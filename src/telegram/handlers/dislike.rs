use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify;
use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service;
use crate::track_status_service::TrackStatusService;

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<bool> {
    if !state.is_spotify_authed().await {
        return Ok(false);
    }

    let track = match spotify::currently_playing(&*state.spotify.read().await).await {
        CurrentlyPlaying::Err(err) => return Err(err),
        CurrentlyPlaying::None(message) => {
            cx.answer(message).send().await?;

            return Ok(true);
        }
        CurrentlyPlaying::Ok(track) => track,
    };

    let track_id = spotify::get_track_id(&track);

    TrackStatusService::set_status(
        &state.app.db,
        &state.user_id,
        &track_id,
        track_status_service::Status::Disliked,
    )
    .await?;

    cx.answer(format!("Disliked {}", spotify::create_track_name(&track)))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Cancel(track_id).into()]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}
