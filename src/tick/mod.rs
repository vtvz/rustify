mod disliked_track;
mod magic;
mod skippage;
mod user;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use tokio::sync::{Semaphore, broadcast};
use tokio::time::Instant;
use tracing::Instrument;
use user::CheckUserResult;

use crate::app::App;
use crate::infrastructure::error_handler;
use crate::services::SpotifyPollingBackoffService;
use crate::spotify::auth::SpotifyAuthService;
use crate::utils;

const CHECK_INTERVAL: Duration = Duration::from_secs(3);
const PARALLEL_CHECKS: usize = 2;

lazy_static::lazy_static! {
    pub static ref PROCESS_TIME_CHANNEL: (
        broadcast::Sender<CheckReport>,
        broadcast::Receiver<CheckReport>
    ) = broadcast::channel(5);
}

#[derive(Clone)]
pub struct CheckReport {
    pub max_process_time: Duration,
    pub users_process_time: Duration,
    pub users_count: usize,
    pub users_checked: usize,
    pub parallel_count: usize,
}

#[tracing::instrument(skip_all)]
async fn process(app: &'static App) -> anyhow::Result<()> {
    let start = Instant::now();

    let user_ids = SpotifyAuthService::get_registered_user_ids(app.db())
        .await
        .context("Get users for processing")?;

    let semaphore = Arc::new(Semaphore::new(PARALLEL_CHECKS));
    let user_ids_len = user_ids.len();
    let mut join_handles = Vec::with_capacity(user_ids_len);

    for user_id in user_ids {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .context("Shouldn't fail")?;

        join_handles.push(tokio::spawn(
            async move {
                let res = user::check(app, &user_id).await;

                drop(permit);

                let checked: anyhow::Result<_> = match res {
                    Err(mut err) => {
                        error_handler::handle(&mut err, app, &user_id, "en").await;

                        Err(err)
                    },
                    Ok(res) => Ok((user_id, res)),
                };

                checked
            }
            .instrument(tracing::info_span!("tick_iteration")),
        ));
    }

    let mut users_checked = 0;
    for handle in join_handles {
        let mut redis_conn = app.redis_conn().await?;

        match handle.await.expect("Shouldn't fail") {
            Ok((user_id, CheckUserResult::Complete)) => {
                users_checked += 1;

                SpotifyPollingBackoffService::reset_idle(&mut redis_conn, &user_id).await?;
            },
            Ok((user_id, CheckUserResult::None(_))) => {
                SpotifyPollingBackoffService::inc_idle(&mut redis_conn, &user_id).await?;

                let suspend_for =
                    SpotifyPollingBackoffService::get_suspend_time(&mut redis_conn, &user_id)
                        .await?;

                dbg!(&suspend_for);

                SpotifyAuthService::suspend_for(app.db(), &[&user_id], suspend_for).await?;
            },
            _ => {},
        }
    }

    let report = CheckReport {
        max_process_time: CHECK_INTERVAL,
        users_process_time: start.elapsed(),
        parallel_count: PARALLEL_CHECKS,
        users_count: user_ids_len,
        users_checked,
    };

    PROCESS_TIME_CHANNEL.0.send(report).ok();

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn check_playing(app: &'static App) {
    utils::tick!(CHECK_INTERVAL, {
        if let Err(err) = process(app).await {
            tracing::error!(err = ?err, "Something went wrong")
        };
    });
}
