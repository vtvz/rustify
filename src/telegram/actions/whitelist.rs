use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::state::{AppState, UserState};
use crate::telegram::actions;
use crate::telegram::keyboards::StartKeyboard;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
    action: String,
    user_id: String,
) -> anyhow::Result<bool> {
    if !app.whitelist().is_admin(state.user_id()) {
        return Ok(false);
    }

    let Ok(user_id_int) = user_id.parse::<i64>() else {
        app.bot()
            .send_message(m.chat.id, "User Id has wrong format. Should be ID")
            .send()
            .await?;

        return Ok(true);
    };

    match action.as_str() {
        "allow" => {
            app.whitelist().allow(app.db(), &user_id).await?;

            app.bot()
                .send_message(
                    m.chat.id,
                    format!(
                        r#"<a href="tg://user?id={}">User</a> added to whitelist"#,
                        user_id
                    ),
                )
                .parse_mode(ParseMode::Html)
                .send()
                .await?;

            app.bot()
                .send_message(
                    ChatId(user_id_int),
                    "Welcome! Admin allowed you to join Rustify family! Enjoy 💃",
                )
                .reply_markup(StartKeyboard::markup())
                .send()
                .await?;

            actions::register::send_register_invite(app, ChatId(user_id_int)).await?;
        },
        "deny" => {
            app.whitelist().deny(app.db(), &user_id).await?;

            app.bot()
                .send_message(
                    m.chat.id,
                    format!(
                        r#"<a href="tg://user?id={}">User</a> denied in whitelist"#,
                        user_id
                    ),
                )
                .parse_mode(ParseMode::Html)
                .send()
                .await?;

            app.bot().send_message(
                ChatId(user_id_int),
                "Sorry... Admin decided to deny you joining to Rustify bot... Maybe a bit later",
            )
            .send()
            .await?;
        },
        _ => {
            app.bot().send_message(
                m.chat.id,
                format!(
                    "Cannot recognise <code>{}</code> action\\. Only <code>allow</code> and <code>deny</code> available",
                    action
                ),
            )
            .parse_mode(ParseMode::Html)
            .send()
            .await?;
        },
    };

    Ok(true)
}