use anyhow::Context as _;
use rspotify::model::TrackId;
use rspotify::prelude::BaseClient as _;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use super::super::inline_buttons::InlineButtons;
use crate::entity::prelude::*;
use crate::spotify::{CurrentlyPlaying, ShortTrack};
use crate::state::{AppState, UserState};
use crate::telegram::handlers::HandleStatus;
use crate::telegram::utils::link_preview_small_top;
use crate::track_status_service::TrackStatusService;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        return Ok(HandleStatus::Skipped);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            app.bot()
                .send_message(m.chat.id, message.to_string())
                .await?;

            return Ok(HandleStatus::Handled);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    TrackStatusService::set_status(app.db(), state.user_id(), track.id(), TrackStatus::Disliked)
        .await?;

    app.bot()
        .send_message(m.chat.id, format!("Disliked {}", track.track_tg_link()))
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Cancel(track.id().into()).into()]
            ],
        )))
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_inline(
    app: &'static AppState,
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

    app.bot()
        .edit_message_text(
            q.from.id,
            q.message.context("Message is empty")?.id(),
            format!("Disliked {}", track.track_tg_link()),
        )
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(track.url()))
        .reply_markup(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Cancel(track.id().into()).into()]
                    ],
        ))
        .await?;

    Ok(())
}
