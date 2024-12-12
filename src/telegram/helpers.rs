use teloxide::prelude::*;
use teloxide::types::{
    ChatId,
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use crate::state::AppState;

pub async fn send_register_invite(app: &'static AppState, chat_id: ChatId) -> anyhow::Result<bool> {
    let url = app.spotify_manager().get_authorize_url().await?;
    app.bot()
        .send_message(
            chat_id,
            "Click this button below and after authentication copy URL from browser and send me",
        )
        .parse_mode(ParseMode::Html)
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
