use chrono::Duration;
use teloxide::payloads::{
    AnswerCallbackQuerySetters as _,
    EditMessageTextSetters as _,
    SendMessageSetters as _,
};
use teloxide::prelude::Requester as _;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{CallbackQuery, ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::services::{SkippageService, UserService};
use crate::telegram::actions;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    to_enable: bool,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    UserService::set_cfg_skippage_enabled(app.db(), state.user_id(), to_enable).await?;

    let days = Duration::seconds(state.user().cfg_skippage_secs).num_days();

    let days_fmt = match days {
        0 => t!("skippage.main.unset-days", locale = state.locale()),
        days => t!(
            "skippage.main.set-days",
            locale = state.locale(),
            days = days
        ),
    };

    app.bot()
        .edit_text(
            &message,
            t!(
                "skippage.main",
                locale = state.locale(),
                command = UserCommandDisplay::Skippage,
                setting = days_fmt,
            ),
        )
        .reply_markup(InlineKeyboardMarkup::new(vec![vec![
            InlineButtons::SkippageEnable(!to_enable).into_inline_keyboard_button(state.locale()),
        ]]))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
    days: String,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::login::send_login_invite(app, state).await?;

        return Ok(HandleStatus::Handled);
    }

    let days = days.parse::<i64>();

    let Ok(days) = days else {
        let days = Duration::seconds(state.user().cfg_skippage_secs).num_days();

        let days_fmt = match days {
            0 => t!("skippage.main.unset-days", locale = state.locale()),
            days => t!(
                "skippage.main.set-days",
                locale = state.locale(),
                days = days
            ),
        };

        let markup = if days == 0 {
            vec![]
        } else {
            #[rustfmt::skip]
            vec![
                vec![
                    InlineButtons::SkippageEnable(!state.user().cfg_skippage_enabled)
                        .into_inline_keyboard_button(state.locale()),
                ]
            ]
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
            .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
                markup,
            )))
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    };

    if !(1..=365).contains(&days) {
        app.bot()
            .send_message(chat_id, t!("skippage.validation", locale = state.locale()))
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let duration = chrono::Duration::days(days);

    UserService::set_cfg_skippage_secs(app.db(), state.user_id(), duration).await?;
    UserService::set_cfg_skippage_enabled(app.db(), state.user_id(), true).await?;

    SkippageService::update_skippage_entries_ttl(
        &mut app.redis_conn().await?,
        state.user_id(),
        state.user().cfg_skippage_secs,
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

    Ok(HandleStatus::Handled)
}
