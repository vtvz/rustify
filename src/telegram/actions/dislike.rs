use rspotify::model::TrackId;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use super::super::inline_buttons::InlineButtons;
use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{RateLimitAction, RateLimitOutput, RateLimitService, TrackStatusService};
use crate::spotify::{CurrentlyPlaying, ShortTrack};
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::DurationPrettyFormat;
use crate::utils::teloxide::CallbackQueryExt as _;

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
        RateLimitService::enforce_limit(&mut redis_conn, state.user_id(), RateLimitAction::Dislike)
            .await?
    {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "rate-limit.dislike",
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

    TrackStatusService::set_status(app.db(), state.user_id(), track.id(), TrackStatus::Disliked)
        .await?;

    let keyboard =
        InlineButtons::from_track_status(TrackStatus::Disliked, track.id(), state.locale());

    app.bot()
        .send_message(m.chat.id, compose_message_text(&track, state.locale()))
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), track_id))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let track = state
        .spotify()
        .await
        .short_track_cached(&mut app.redis_conn().await?, TrackId::from_id(track_id)?)
        .await?;

    TrackStatusService::set_status(app.db(), state.user_id(), track_id, TrackStatus::Disliked)
        .await?;

    let keyboard =
        InlineButtons::from_track_status(TrackStatus::Disliked, track.id(), state.locale());

    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    app.bot()
        .edit_text(&message, compose_message_text(&track, state.locale()))
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}

fn compose_message_text(track: &ShortTrack, locale: &str) -> String {
    t!(
        "actions.dislike",
        locale = locale,
        track_link = track.track_tg_link()
    )
    .to_string()
}
