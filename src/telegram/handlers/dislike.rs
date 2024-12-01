use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup, ReplyParameters};

use super::super::inline_buttons::InlineButtons;
use crate::entity::prelude::*;
use crate::spotify::CurrentlyPlaying;
use crate::state::{AppState, UserState};
use crate::track_status_service::TrackStatusService;

pub async fn handle(
    app_state: &'static AppState,
    state: &UserState,
    bot: &Bot,
    m: &Message,
) -> anyhow::Result<bool> {
    if !state.is_spotify_authed().await {
        return Ok(false);
    }

    let track = match CurrentlyPlaying::get(&*state.spotify().await).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            bot.send_message(m.chat.id, message.to_string())
                .send()
                .await?;

            return Ok(true);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    let track_id = track.id();

    TrackStatusService::set_status(
        app_state.db(),
        state.user_id(),
        track_id,
        TrackStatus::Disliked,
    )
    .await?;

    bot.send_message(m.chat.id, format!("Disliked {}", track.track_tg_link()))
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Cancel(track_id.into()).into()]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}
