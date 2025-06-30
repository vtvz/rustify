use anyhow::Context;
use strum_macros::Display;

use super::skippage;
use crate::app::App;
use crate::entity::prelude::*;
use crate::spotify::CurrentlyPlaying;
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;
use crate::{error_handler, queue, rickroll, spotify};

#[allow(dead_code)]
#[derive(Clone, Display)]
pub enum CheckUserResult {
    #[strum(serialize = "Skip same track")]
    SkipSame,
    #[strum(serialize = "Complete check")]
    Complete,
    #[strum(serialize = "Current track is on pause {0}")]
    None(spotify::CurrentlyPlayingNoneReason),
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn check(app: &'static App, user_id: &str) -> anyhow::Result<CheckUserResult> {
    let res = app.user_state(user_id).await.context("Get user state");

    let state = match res {
        Err(mut err) => {
            error_handler::handle(&mut err, app, user_id, "en").await;

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

    rickroll::queue(app, &state).await.ok();

    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let skippage_skipped = skippage::handle(app, &state, &track, &user).await?;

    if skippage_skipped {
        return Ok(CheckUserResult::Complete);
    }

    super::magic::handle(app, &state, &track, context.as_ref(), &user)
        .await
        .ok();

    let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

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
