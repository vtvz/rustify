mod bad_words;
mod disliked_track;
mod errors;
mod user;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::Timelike;
use tokio::sync::{broadcast, Semaphore};
use tokio::time::Instant;
use tracing::Instrument;
use user::CheckUserResult;

use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::utils::Clock;
use crate::{spotify, state, utils, UserService};

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
async fn process(app_state: &'static state::AppState) -> anyhow::Result<()> {
    let start = Instant::now();

    let user_ids = SpotifyAuthService::get_registered(app_state.db())
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
            let res = user::check(app_state, &user_id).await;
            drop(permit);

            // TODO Refactor this mess...
            let checked: anyhow::Result<_> = match res {
                Err(mut err) => {
                    match spotify::Error::from_anyhow(&mut err).await {
                        Err(_) | Ok(None) => {
                            tracing::error!(user_id = %user_id, err = ?err, "Something went wrong");

                            Err(err)
                        },
                        Ok(Some(spotify::Error::Regular(serr))) => {
                            if serr.error.status < 500 {
                                tracing::error!(user_id = %user_id, err = ?serr, "Regular Spotify Error Happened");
                            }

                            if serr.error.status == 403 && serr.error.message == "Spotify is unavailable in this country" {
                                UserService::set_status(app_state.db(), &user_id, UserStatus::Forbidden).await?;
                            }

                            Err(err)
                        },
                        Ok(Some(spotify::Error::Auth(serr))) => {
                            tracing::error!(user_id = %user_id, err = ?serr, "Auth Spotify Error Happened");

                            Err(err)
                        },
                    }
                },
                Ok(res) => Ok((user_id, res)),
            };

            checked
        }.instrument(tracing::info_span!("tick_ineration"))));
    }

    let mut users_checked = 0;
    let mut users_to_suspend = Vec::new();
    for handle in join_handles {
        match handle.await.expect("Shouldn't fail") {
            Ok((_, CheckUserResult::Complete)) => {
                users_checked += 1;
            },
            Ok((user_id, CheckUserResult::None(_))) => {
                users_to_suspend.push(user_id);
            },
            _ => {},
        }
    }

    // TODO: Prevent overflow on large amount of users
    if !users_to_suspend.is_empty() {
        let suspend_until = Clock::now() + chrono::Duration::seconds(6);

        let roundup = suspend_until.second() as i64 % 5;
        let roundup = if roundup == 0 { 0 } else { 5 - roundup };

        let suspend_until = suspend_until + chrono::Duration::seconds(roundup);

        SpotifyAuthService::suspend_until(
            app_state.db(),
            &users_to_suspend
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>(),
            suspend_until,
        )
        .await?;
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
            tracing::error!(err = ?err, "Something went wrong")
        };
    });
}
