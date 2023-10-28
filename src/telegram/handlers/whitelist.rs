use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::state::UserState;
use crate::telegram::helpers;
use crate::telegram::keyboards::StartKeyboard;

pub async fn handle(
    m: &Message,
    bot: &Bot,
    state: &UserState,
    action: String,
    user_id: String,
) -> anyhow::Result<bool> {
    if !state.app.whitelist().is_admin(&state.user_id) {
        return Ok(false);
    }

    let Ok(user_id_int) = user_id.parse::<i64>() else {
        bot.send_message(m.chat.id, "User Id has wrong format. Should be ID")
            .send()
            .await?;

        return Ok(true);
    };

    match action.as_str() {
        "allow" => {
            state
                .app
                .whitelist()
                .allow(state.app.db(), &user_id)
                .await?;

            bot.send_message(
                m.chat.id,
                format!("[User](tg://user?id={}) added to whitelist", user_id),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await?;

            bot.send_message(
                ChatId(user_id_int),
                "Welcome! Admin allowed you to join Rustify family! Enjoy 💃",
            )
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;

            helpers::send_register_invite(ChatId(user_id_int), bot, state).await?;
        },
        "deny" => {
            state.app.whitelist().deny(state.app.db(), &user_id).await?;

            bot.send_message(
                m.chat.id,
                format!("[User](tg://user?id={}) denied in whitelist", user_id),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await?;

            bot.send_message(
                ChatId(user_id_int),
                "Sorry... Admin decided to deny you joining to Rustify bot... Maybe a bit later",
            )
            .send()
            .await?;
        },
        _ => {
            bot.send_message(
                m.chat.id,
                format!(
                    "Cannot recognise `{}` action\\. Only `allow` and `deny` available",
                    action
                ),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await?;
        },
    };

    Ok(true)
}
