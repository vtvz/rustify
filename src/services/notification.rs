use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode, User};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::UserService;

pub struct NotificationService;

impl NotificationService {
    async fn notify_admins(app: &'static App, text: &str) -> anyhow::Result<()> {
        let admins = UserService::get_users_by_role(app.db(), UserRole::Admin).await?;

        for admin in admins {
            let chat_id = ChatId(admin.id.parse()?);

            if let Err(err) = app
                .bot()
                .send_message(chat_id, text)
                .parse_mode(ParseMode::Html)
                .await
            {
                tracing::warn!(
                    admin_id = %admin.id,
                    error = ?err,
                    "Failed to send admin notification"
                );
            }
        }

        Ok(())
    }

    pub async fn notify_user_joined(
        app: &'static App,
        user: Option<&User>,
        ref_code: Option<String>,
    ) -> anyhow::Result<()> {
        let user = user
            .map_or("Unknown".into(), |user| {
                format!(
                    "<code>{id}</code> <a href=\"tg://user?id={id}\">link</a> {name} {surname} {username}\nRef Code: {ref_code}",
                    id = user.id,
                    name = user.first_name,
                    surname = user.last_name.as_deref().unwrap_or_default(),
                    username = user
                        .username
                        .as_deref()
                        .map(|username| format!("(@{username})"))
                        .unwrap_or_default(),
                    ref_code = ref_code
                        .map_or("<i>None</i>".into(), |text| format!("<code>{text}</code>")),
                )
                .trim()
                .to_string()
            });

        let message = format!("🆕 <b>New user joined</b>\n\n{user}");

        Self::notify_admins(app, &message).await
    }

    pub async fn notify_spotify_connected(
        app: &'static App,
        user: &UserModel,
    ) -> anyhow::Result<()> {
        let message = format!(
            "✅ <b>User connected Spotify</b>\n\nName: {name}\nID: <code>{id}</code> <a href=\"tg://user?id={id}\">link</a>",
            id = user.id,
            name = user.name
        );

        Self::notify_admins(app, &message).await
    }
}
