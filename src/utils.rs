use again::RetryPolicy;
use lazy_static::lazy_static;
use std::time::Duration;
use tokio::sync::broadcast;

macro_rules! tick {
    ($period:expr, $code:block) => {
        let __period = $period;
        let mut __interval = ::tokio::time::interval(__period);
        loop {
            ::tokio::select! {
                _ = __interval.tick() => {},
                _ = $crate::utils::ctrl_c() => {
                    break;
                },
            }

            let __start = ::tokio::time::Instant::now();
            $code;
            let __diff = ::tokio::time::Instant::now().duration_since(__start);

            if (__diff > __period) {
                ::tracing::warn!(
                    diff = (__diff - __period).as_secs_f64(),
                    unit = "s",
                    "Task took a bit more time than allowed"
                );
            }
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

pub async fn listen_for_ctrl_c() {
    tokio::signal::ctrl_c().await.ok();

    KILL.0.send(()).ok();
}

pub async fn ctrl_c() {
    KILL.0.subscribe().recv().await.ok();
}
