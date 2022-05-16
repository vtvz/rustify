mod bad_words;
mod disliked_track;
mod errors;
mod user;

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Semaphore};
use tokio::time::Instant;
use user::CheckUserResult;

use crate::errors::Context;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::{state, utils, GenericResult};

const CHECK_INTERVAL: u64 = 3;
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
async fn process(app_state: &'static state::AppState) -> GenericResult<()> {
    let start = Instant::now();

    let user_ids = SpotifyAuthService::get_registered(&app_state.db)
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

        join_handles.push(tokio::spawn(async move {
            let user_id = user_id.as_str();
            let res = user::check(app_state, user_id).await;
            drop(permit);
            let checked = match res {
                Err(err) => {
                    tracing::error!(user_id, err = ?err.anyhow(), "Something went wrong");
                    false
                },
                Ok(CheckUserResult::Complete) => true,
                _ => false,
            };

            checked
        }));
    }

    let mut users_checked = 0;
    for handle in join_handles {
        if handle.await.expect("Shouldn't fail") {
            users_checked += 1;
        }
    }

    let report = CheckReport {
        max_process_time: Duration::from_secs(CHECK_INTERVAL),
        users_process_time: start.elapsed(),
        parallel_count: PARALLEL_CHECKS,
        users_count: user_ids_len,
        users_checked,
    };

    PROCESS_TIME_CHANNEL.0.send(report).ok();

    Ok(())
}

#[tracing::instrument(skip_all)]
pub async fn check_playing(app_state: &'static state::AppState) {
    utils::tick!(Duration::from_secs(CHECK_INTERVAL), {
        if let Err(err) = process(app_state).await {
            tracing::error!(err = ?err.anyhow(), "Something went wrong")
        };
    });
}
