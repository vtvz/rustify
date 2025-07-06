use anyhow::Context as _;
use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use super::super::inline_buttons::InlineButtons;
use crate::app::App;
use crate::entity::prelude::*;
use crate::spotify::{CurrentlyPlaying, ShortTrack};
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::utils::link_preview_small_top;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, m.chat.id, state.locale()).await?;

        return Ok(HandleStatus::Handled);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            app.bot()
                .send_message(m.chat.id, message.localize(state.locale()))
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
        .send_message(m.chat.id, compose_message(&track, state.locale()))
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    track_id: &str,
) -> anyhow::Result<()> {
    let track = state
        .spotify()
        .await
        .track(TrackId::from_id(track_id)?, None)
        .await?;

    let track = ShortTrack::new(track);

    TrackStatusService::set_status(app.db(), state.user_id(), track_id, TrackStatus::Disliked)
        .await?;

    let keyboard =
        InlineButtons::from_track_status(TrackStatus::Disliked, track.id(), state.locale());

    app.bot()
        .edit_message_text(
            q.from.id,
            q.message.context("Message is empty")?.id(),
            compose_message(&track, state.locale()),
        )
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .await?;

    Ok(())
}

fn compose_message(track: &ShortTrack, locale: &str) -> String {
    t!(
        "actions.dislike",
        locale = locale,
        track_link = track.track_tg_link()
    )
    .to_string()
}
