use chrono::Duration;
use indoc::formatdoc;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;

use crate::app::App;
use crate::skippage_service::SkippageService;
use crate::telegram::handlers::HandleStatus;
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

    let Ok(days) = days else {
        let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

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
                        Pass number of days to remember played tracks.
                        Pass zero to disable this function.
                        Number should be 365 or less.
                        Changing days will clear remembered songs.
                        Current setting: {}
                    ",
                    days_fmt
                ),
            )
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

    let deleted =
        SkippageService::delete_skipage_entries(&mut app.redis_conn().await?, state.user_id())
            .await?;

    app.bot()
        .send_message(
            chat_id,
            format!("All tracks you've listened within {days} days will be skipped.\nAll listen history were deleted ({deleted} entries)"),
        )
        .await?;

    Ok(HandleStatus::Handled)
}
