use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;

pub async fn handle(
    app: &'static App,
    m: &Message,
    action: String,
    user_id: String,
) -> anyhow::Result<HandleStatus> {
    let Ok(user_id_int) = user_id.parse::<i64>() else {
        app.bot()
            .send_message(m.chat.id, "User Id has wrong format. Should be ID")
            .await?;

        return Ok(HandleStatus::Handled);
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
                .await?;

            app.bot()
                .send_message(
                    ChatId(user_id_int),
                    "Welcome! Admin allowed you to join Rustify family! Enjoy 💃",
                )
                .await?;

            actions::register::send_register_invite(app, ChatId(user_id_int), "en").await?;
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
                .await?;

            app.bot().send_message(
                ChatId(user_id_int),
                "Sorry... Admin decided to deny you joining to Rustify bot... Maybe a bit later",
            )
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
            .await?;
        },
    };

    Ok(HandleStatus::Handled)
}
