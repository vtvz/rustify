use anyhow::Result;
use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use crate::state::UserState;

pub async fn send_register_invite(m: &Message, bot: &Bot, state: &UserState) -> Result<bool> {
    let url = state.app.spotify_manager.get_authorize_url().await?;
    bot.send_message(
        m.chat.id,
        "Click this button below and after authentication copy URL from browser and send me",
    )
    .parse_mode(ParseMode::MarkdownV2)
    .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
        #[rustfmt::skip]
            vec![
                vec![InlineKeyboardButton {
                    text: "Login with Spotify".into(),
                    kind: InlineKeyboardButtonKind::Url(url.parse()?),
                }]
            ],
    )))
    .send()
    .await?;

    Ok(true)
}
