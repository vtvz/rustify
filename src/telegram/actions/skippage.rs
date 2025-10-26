use anyhow::Context;
use chrono::Duration;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::types::{CallbackQuery, ChatId, InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::services::{SkippageService, UserService};
use crate::telegram::actions;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::user::UserState;

pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    to_enable: bool,
) -> anyhow::Result<()> {
    let chat_id = q.from.id;
    let message_id = q.message.clone().context("Message is empty")?.id();

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
        .edit_message_text(
            chat_id,
            message_id,
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

pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
    days: String,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id, state.locale()).await?;

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
