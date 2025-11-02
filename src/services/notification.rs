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

    pub async fn notify_user_joined(app: &'static App, user: Option<&User>) -> anyhow::Result<()> {
        let user = user
            .map(|user| {
                format!(
                    "<code>{}</code> {} {} {}",
                    user.id,
                    user.first_name,
                    user.last_name.as_deref().unwrap_or_default(),
                    user.username
                        .as_deref()
                        .map(|username| format!("(@{username})"))
                        .unwrap_or_default()
                )
                .trim()
                .to_string()
            })
            .unwrap_or("Unknown".into());

        let message = format!("🆕 <b>New user joined</b>\n\n{}", user);

        Self::notify_admins(app, &message).await
    }

    pub async fn notify_spotify_connected(
        app: &'static App,
        user: &UserModel,
    ) -> anyhow::Result<()> {
        let message = format!(
            "✅ <b>User connected Spotify</b>\n\nName: {}\nID: <code>{}</code>",
            user.name, user.id
        );

        Self::notify_admins(app, &message).await
    }
}
