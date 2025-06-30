use chrono::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::skippage_service::SkippageService;
use crate::telegram::actions;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;
use crate::user_service::UserService;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
    days: String,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id).await?;

        return Ok(HandleStatus::Handled);
    }

    let days = days.parse::<i64>();
    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let Ok(days) = days else {
        let days = Duration::seconds(user.cfg_skippage_secs).num_days();
        let days_fmt = match days {
            0 => t!("skippage.main.disabled", locale = state.locale()),
            1 => t!("skippage.main.one", locale = state.locale()),
            days => t!("skippage.main.more", locale = state.locale(), days = days),
        };

        app.bot()
            .send_message(
                chat_id,
                t!(
                    "skippage.main",
                    locale = state.locale(),
                    command = UserCommandDisplay::Skippage,
                    setting = days_fmt,
                ),
            )
            .reply_markup(StartKeyboard::markup(state.locale()))
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    };

    if !(0..=365).contains(&days) {
        app.bot()
            .send_message(chat_id, t!("skippage.validation", locale = state.locale()))
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let duration = chrono::Duration::days(days);

    UserService::set_cfg_skippage_secs(app.db(), state.user_id(), duration).await?;

    if days > 0 {
        SkippageService::update_skippage_entries_ttl(
            &mut app.redis_conn().await?,
            state.user_id(),
            user.cfg_skippage_secs,
            duration.num_seconds(),
        )
        .await?;
        app.bot()
            .send_message(
                chat_id,
                t!("skippage.updated", locale = state.locale(), days = days),
            )
            .parse_mode(ParseMode::Html)
            .await?;
    } else {
        app.bot()
            .send_message(chat_id, t!("skippage.disabled", locale = state.locale()))
            .parse_mode(ParseMode::Html)
            .await?;
    }

    Ok(HandleStatus::Handled)
}
