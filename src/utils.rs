use std::collections::HashMap;
use std::time::Duration;

use again::RetryPolicy;
use chrono::{NaiveDateTime, SubsecRound, Utc};
use lazy_static::lazy_static;
use tokio::sync::{RwLock, broadcast};
use tokio::time::Instant;

lazy_static! {
    pub static ref TICK_STATUS: RwLock<HashMap<(&'static str, Duration), Instant>> =
        RwLock::new(HashMap::new());
}

#[derive(Clone, Debug)]
pub struct TickHealthStatus {
    pub lagging: Vec<&'static str>,
    pub unhealthy: Vec<&'static str>,
    pub total: usize,
}

pub async fn tick_health() -> TickHealthStatus {
    let hash = TICK_STATUS.read().await;

    let now = Instant::now();

    let mut lagging = vec![];
    let mut unhealthy = vec![];
    for ((module, period), last) in hash.iter() {
        let diff = now.duration_since(*last);

        if diff >= (*period * 3) {
            unhealthy.push(*module);
        } else if diff >= (*period * 2) {
            lagging.push(*module);
        }
    }

    TickHealthStatus {
        unhealthy,
        lagging,
        total: hash.len(),
    }
}

macro_rules! tick {
    ($period:expr, $code:block) => {
        let __period = $period;
        let __health_check_key = (concat!(module_path!(), ":", line!()), __period);
        let mut __interval = ::tokio::time::interval(__period);
        let mut __iteration: u64 = 0;
        loop {
            use ::tracing::Instrument;

            ::tokio::select! {
                _ = __interval.tick() => {},
                _ = $crate::utils::ctrl_c() => {
                    ::tracing::debug!(tick_iteration = __iteration, "Received terminate signal. Stop processing");
                    $crate::utils::TICK_STATUS
                        .write()
                        .await
                        .remove(&__health_check_key);
                    break;
                },
            }

            // __interval.tick() can lag behind
            let __start = ::tokio::time::Instant::now();

            {
                $crate::utils::TICK_STATUS
                    .write()
                    .await
                    .insert(__health_check_key, __start);
            }

            async { $code }
                .instrument(tracing::info_span!("tick", tick_iteration = __iteration))
                .await;

            let __diff = __start.elapsed();

            if (__diff > __period) {
                ::tracing::warn!(
                    tick_iteration = __iteration,
                    diff = (__diff - __period).as_secs_f64(),
                    unit = "s",
                    "Task took a bit more time than allowed"
                );
            }

            __iteration += 1;
        }
    };
}

pub(crate) use tick;

pub async fn retry<T>(task: T) -> Result<T::Item, T::Error>
where
    T: again::Task,
{
    let policy = RetryPolicy::exponential(Duration::from_millis(100))
        .with_jitter(true)
        .with_max_delay(Duration::from_secs(50))
        .with_max_retries(10);

    policy.retry(task).await
}

lazy_static! {
    static ref KILL: (broadcast::Sender<()>, broadcast::Receiver<()>) = broadcast::channel(1);
}

static mut KILLED: bool = false;

pub async fn listen_for_ctrl_c() {
    tokio::signal::ctrl_c().await.ok();

    KILL.0.send(()).ok();

    unsafe { KILLED = true };
}

pub async fn ctrl_c() {
    if unsafe { KILLED } {
        return;
    }

    KILL.0.subscribe().recv().await.ok();
}

pub struct Clock;

impl Clock {
    pub fn now() -> NaiveDateTime {
        Utc::now().naive_local().round_subsecs(0)
    }
}
