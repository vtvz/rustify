use anyhow::Context;
use strum_macros::Display;

use crate::app::App;
use crate::entity::prelude::*;
use crate::spotify::{CurrentlyPlaying, SpotifyError};
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;
use crate::{queue, spotify};

#[allow(dead_code)]
#[derive(Clone, Display)]
pub enum CheckUserResult {
    #[strum(serialize = "Skip same track")]
    SkipSame,
    #[strum(serialize = "Complete check")]
    Complete,
    #[strum(serialize = "Current track is on pause {1}")]
    None(spotify::CurrentlyPlayingNoneReason),
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn check(app: &'static App, user_id: &str) -> anyhow::Result<CheckUserResult> {
    let res = app.user_state(user_id).await.context("Get user state");

    let state = match res {
        Err(mut err) => {
            let Some(response) = SpotifyError::extract_response(&mut err) else {
                return Err(err);
            };

            if let Err(err) =
                crate::telegram::errors::handle_too_many_requests(app.db(), user_id, response).await
            {
                tracing::error!(err = ?err, "Something went wrong");
            }

            return Err(err);
        },
        Ok(state) => state,
    };

    let playing = CurrentlyPlaying::get(&*state.spotify().await).await;

    let (track, context) = match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        },
        CurrentlyPlaying::None(reason) => {
            return Ok(CheckUserResult::None(reason));
        },
        CurrentlyPlaying::Ok(track, context) => (track, context),
    };

    let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    match status {
        TrackStatus::Disliked => {
            if user.cfg_skip_tracks {
                super::disliked_track::handle(app, &state, &track, context.as_ref()).await?;
            }
        },
        TrackStatus::None => {
            if user.cfg_check_profanity {
                let changed = UserService::sync_current_playing(
                    app.redis_conn().await?,
                    state.user_id(),
                    track.id(),
                )
                .await?;

                if !changed {
                    return Ok(CheckUserResult::SkipSame);
                }

                queue::profanity_check::queue(app.redis_conn().await?, state.user_id(), &track)
                    .await
                    .context("Check bad words")?;
            }
        },
        TrackStatus::Ignore => {},
    }

    Ok(CheckUserResult::Complete)
}
