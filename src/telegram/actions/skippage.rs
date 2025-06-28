use chrono::Duration;
use indoc::formatdoc;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::skippage_service::SkippageService;
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
        return Ok(HandleStatus::Skipped);
    }

    let days = days.parse::<i64>();
    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let Ok(days) = days else {
        let days = Duration::seconds(user.cfg_skippage_secs).num_days();
        let days_fmt = match days {
            0 => "disabled".to_owned(),
            1 => "1 day".to_owned(),
            days => format!("{} days", days),
        };

        app.bot()
            .send_message(
                chat_id,
                formatdoc!(
                    "
                        Pass number of days to remember played tracks like that:
                        <code>/{} 7</code>
                        Pass zero to disable this function:
                        <code>/{} 0</code>
                        Number should be 365 or less.
                        Current setting: {}
                    ",
                    UserCommandDisplay::Skippage,
                    UserCommandDisplay::Skippage,
                    days_fmt,
                ),
            )
            .reply_markup(StartKeyboard::markup())
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    };

    if !(0..=365).contains(&days) {
        app.bot()
            .send_message(chat_id, "Number should be possitive and 365 or less")
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
                format!("All tracks you've listened within {days} days will be skipped"),
            )
            .await?;
    } else {
        app.bot()
            .send_message(chat_id, "Slippage is disabled")
            .await?;
    }

    Ok(HandleStatus::Handled)
}
