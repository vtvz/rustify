use teloxide::prelude::*;
use teloxide::types::{
    ChatId,
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use crate::errors::GenericResult;
use crate::state::UserState;

pub async fn send_register_invite(
    chat_id: ChatId,
    bot: &Bot,
    state: &UserState,
) -> GenericResult<bool> {
    let url = state.app.spotify_manager.get_authorize_url().await?;
    bot.send_message(
        chat_id,
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
