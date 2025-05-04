mod disliked_track;
mod user;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use tokio::sync::{Semaphore, broadcast};
use tokio::time::Instant;
use tracing::Instrument;
use user::CheckUserResult;

use crate::app::App;
use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::user_service::UserService;
use crate::{rickroll, spotify, utils};

const CHECK_INTERVAL: Duration = Duration::from_secs(3);
const PARALLEL_CHECKS: usize = 2;
const SUSPEND_FOR_ON_IDLE: chrono::Duration = chrono::Duration::seconds(10);

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

        join_handles.push(tokio::spawn(async move {
            let res = user::check(app, &user_id).await;
            rickroll::queue(app, &user_id).await.ok();

            drop(permit);

            // TODO: Refactor this mess...
            let checked: anyhow::Result<_> = match res {
                Err(mut err) => {
                    match spotify::SpotifyError::from_anyhow(&mut err).await {
                        Err(_) | Ok(None) => {
                            tracing::error!(user_id = %user_id, err = ?err, "Something went wrong");

                            Err(err)
                        },
                        Ok(Some(spotify::SpotifyError::Regular(serr))) => {
                            if serr.error.status < 500 {
                                tracing::error!(user_id = %user_id, err = ?serr, "Regular Spotify Error Happened");
                            }

                            if serr.error.status == 403 && serr.error.message == "Spotify is unavailable in this country" {
                                UserService::set_status(app.db(), &user_id, UserStatus::Forbidden).await?;
                            }

                            Err(err)
                        },
                        Ok(Some(spotify::SpotifyError::Auth(serr))) => {
                            tracing::error!(user_id = %user_id, err = ?serr, "Auth Spotify Error Happened");

                            Err(err)
                        },
                    }
                },
                Ok(res) => Ok((user_id, res)),
            };

            checked
        }.instrument(tracing::info_span!("tick_iteration"))));
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
        SpotifyAuthService::suspend_for(
            app.db(),
            &users_to_suspend
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>(),
            SUSPEND_FOR_ON_IDLE,
        )
        .await?;
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
