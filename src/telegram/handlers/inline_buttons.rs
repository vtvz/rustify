use anyhow::Context;
use rspotify::model::TrackId;
use rspotify::prelude::*;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::entity::prelude::*;
use crate::spotify::ShortTrack;
use crate::state::{AppState, UserState};
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::track_status_service::TrackStatusService;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    if !state.is_spotify_authed().await {
        if let Some(id) = q.inline_message_id {
            app.bot()
                .answer_callback_query(id)
                .text("You need to register first")
                .send()
                .await?;
        }

        return Ok(());
    }

    let data = q.data.context("Callback needs data")?;

    let button: InlineButtons = data.parse()?;

    match button {
        InlineButtons::Cancel(id) => {
            let track = state
                .spotify()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;
            let track = ShortTrack::new(track);

            TrackStatusService::set_status(app.db(), state.user_id(), &id, TrackStatus::None)
                .await?;

            app.bot()
                .edit_message_text(
                    q.from.id,
                    q.message.context("Message is empty")?.id(),
                    format!("Dislike cancelled for {}", track.track_tg_link()),
                )
                .link_preview_options(link_preview_small_top(track.url()))
                .parse_mode(ParseMode::Html)
                .reply_markup(InlineKeyboardMarkup::new(
                    #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Dislike(id).into()]
                    ],
                ))
                .send()
                .await?;
        },
        InlineButtons::Dislike(id) => {
            let track = state
                .spotify()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;

            let track = ShortTrack::new(track);

            TrackStatusService::set_status(app.db(), state.user_id(), &id, TrackStatus::Disliked)
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
                        vec![InlineButtons::Cancel(id).into()]
                    ],
                ))
                .send()
                .await?;
        },
        InlineButtons::Ignore(id) => {
            let track = state
                .spotify()
                .await
                .track(TrackId::from_id(&id)?, None)
                .await?;
            let track = ShortTrack::new(track);

            TrackStatusService::set_status(app.db(), state.user_id(), &id, TrackStatus::Ignore)
                .await?;

            app.bot()
                .edit_message_text(
                    q.from.id,
                    q.message.context("Message is empty")?.id(),
                    format!(
                        "Bad words of {} will be forever ignored",
                        track.track_tg_link()
                    ),
                )
                .link_preview_options(link_preview_small_top(track.url()))
                .parse_mode(ParseMode::Html)
                .reply_markup(InlineKeyboardMarkup::new(
                    #[rustfmt::skip]
                    vec![
                        vec![InlineButtons::Cancel(id).into()]
                    ],
                ))
                .send()
                .await?;
        },
    }

    Ok(())
}
