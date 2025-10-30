use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::UserService;

pub struct NotificationService;

impl NotificationService {
    async fn notify_admins(app: &'static App, message: &str) -> anyhow::Result<()> {
        let admins = UserService::get_users_by_role(app.db(), UserRole::Admin).await?;

        for admin in admins {
            let chat_id = ChatId(admin.id.parse()?);

            if let Err(err) = app
                .bot()
                .send_message(chat_id, message)
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

    pub async fn notify_user_joined(app: &'static App, user: &UserModel) -> anyhow::Result<()> {
        let message = format!(
            "🆕 <b>New user joined</b>\n\nName: {}\nID: <code>{}</code>",
            user.name, user.id
        );

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
