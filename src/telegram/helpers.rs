use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify;
use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service;
use crate::CurrentlyPlaying::Error;
use crate::USER_ID;

pub async fn handle_dislike(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState<'static>,
) -> Result<bool> {
    let track = match spotify::currently_playing(&state.spotify).await {
        Error(error) => return Err(error),
        CurrentlyPlaying::None(_) => return Ok(true),
        CurrentlyPlaying::Ok(track) => track,
    };

    let track_id = spotify::get_track_id(&track);

    track_status_service::set_status(
        &state.app.db,
        USER_ID.to_string(),
        track_id.clone(),
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
