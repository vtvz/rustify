use chrono::Duration;
use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{NotificationService, UserService};
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::{LanguageKeyboard, StartKeyboard};
use crate::user::UserState;
use crate::utils::Clock;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if state.is_spotify_authed().await {
        let was_inactive = matches!(state.user().status, UserStatus::Inactive);

        UserService::set_status(app.db(), state.user_id(), UserStatus::Active).await?;

        if was_inactive {
            tracing::info!(user_id = state.user_id(), "User were reactivated");
        }

        let text = if was_inactive {
            t!("status.reactivated", locale = state.locale())
        } else {
            t!("actions.here-is-your-keyboard", locale = state.locale())
        };

        app.bot()
            .send_message(m.chat.id, text)
            .reply_markup(StartKeyboard::markup(state.locale()))
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    app.bot()
        .send_message(m.chat.id, t!("language.command", locale = state.locale()))
        .reply_markup(LanguageKeyboard::markup())
        .await?;

    if (Clock::now() - state.user().created_at) < Duration::minutes(1) {
        if let Err(err) = NotificationService::notify_user_joined(app, m.from.as_ref()).await {
            tracing::error!(err = ?err, user_id = state.user_id(), "Failed to notify admins about joined user");
        };
    };

    Ok(HandleStatus::Handled)
}
