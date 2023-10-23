use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use super::super::inline_buttons::InlineButtons;
use crate::entity::prelude::*;
use crate::spotify;
use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::track_status_service::TrackStatusService;

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    if !state.is_spotify_authed().await {
        return Ok(false);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify.read().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            bot.send_message(m.chat.id, message.to_string())
                .send()
                .await?;

            return Ok(true);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    let track_id = spotify::utils::get_track_id(&track);

    TrackStatusService::set_status(
        &state.app.db,
        &state.user_id,
        &track_id,
        TrackStatus::Disliked,
    )
    .await?;

    bot.send_message(
        m.chat.id,
        format!("Disliked {}", spotify::utils::create_track_tg_link(&track)),
    )
    .reply_to_message_id(m.id)
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
