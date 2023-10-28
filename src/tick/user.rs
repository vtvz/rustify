use anyhow::Context;
use strum_macros::Display;

use crate::entity::prelude::*;
use crate::spotify::{CurrentlyPlaying, Error};
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;
use crate::{lyrics, spotify, state};

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
pub async fn check(
    app_state: &'static state::AppState,
    user_id: &str,
) -> anyhow::Result<CheckUserResult> {
    let res = app_state
        .user_state(user_id)
        .await
        .context("Get user state");

    let state = match res {
        Err(mut err) => {
            let Some(response) = Error::extract_response(&mut err) else {
                return Err(err);
            };

            if let Err(err) =
                super::errors::handle_too_many_requests(app_state.db(), user_id, response).await
            {
                tracing::error!(err = ?err, "Something went wrong");
            }

            return Err(err);
        },
        Ok(state) => state,
    };

    let playing = CurrentlyPlaying::get(&*state.spotify.read().await).await;

    let (track, context) = match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        },
        CurrentlyPlaying::None(reason) => {
            return Ok(CheckUserResult::None(reason));
        },
        CurrentlyPlaying::Ok(track, context) => (track, context),
    };

    let status = TrackStatusService::get_status(
        state.app.db(),
        &state.user_id,
        &spotify::utils::get_track_id(&track),
    )
    .await;

    match status {
        TrackStatus::Disliked => {
            super::disliked_track::handle(&state, &track, context.as_ref()).await?;
        },
        TrackStatus::None => {
            let changed = UserService::sync_current_playing(
                state.app.db(),
                &state.user_id,
                &spotify::utils::get_track_id(&track),
            )
            .await?;

            if !changed {
                return Ok(CheckUserResult::SkipSame);
            }

            let res = super::bad_words::check(&state, &track)
                .await
                .context("Check bad words");

            match res {
                Ok(res) => {
                    UserService::increase_stats_query(&state.user_id)
                        .lyrics(
                            1,
                            res.profane as u32,
                            matches!(res.provider, Some(lyrics::Provider::Genius)) as u32,
                            matches!(res.provider, Some(lyrics::Provider::Musixmatch)) as u32,
                        )
                        .exec(state.app.db())
                        .await?;
                },
                Err(err) => {
                    tracing::error!(
                        err = ?err,
                        track_id = %spotify::utils::get_track_id(&track),
                        track_name = %spotify::utils::create_track_name(&track),
                        "Error occurred on checking bad words",
                    )
                },
            }
        },
        TrackStatus::Ignore => {},
    }

    Ok(CheckUserResult::Complete)
}
