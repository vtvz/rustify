use rspotify::prelude::OAuthClient as _;
use teloxide::prelude::*;

use crate::app::App;
use crate::services::{RateLimitAction, RateLimitOutput, RateLimitService};
use crate::spotify::CurrentlyPlaying;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::DurationPrettyFormat as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::login::send_login_invite(app, state).await?;

        return Ok(HandleStatus::Handled);
    }

    let mut redis_conn = app.redis_conn().await?;

    if let RateLimitOutput::NeedToWait(duration) =
        RateLimitService::enforce_limit(&mut redis_conn, state.user_id(), RateLimitAction::Like)
            .await?
    {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "rate-limit.like",
                    duration = duration.pretty_format(),
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let track = match state.spotify().await.current_playing_wrapped().await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(reason) => {
            app.bot()
                .send_message(m.chat.id, reason.localize(state.locale()))
                .await?;

            return Ok(HandleStatus::Handled);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    state
        .spotify()
        .await
        .current_user_saved_tracks_add([track.raw_id().clone()])
        .await?;

    app.bot()
        .send_message(m.chat.id, format!("Liked {}", track.track_tg_link()))
        .reply_markup(StartKeyboard::markup(state.locale()))
        .link_preview_options(link_preview_small_top(track.url()))
        .await?;

    Ok(HandleStatus::Handled)
}
