#![allow(clippy::explicit_auto_deref)]
#![warn(clippy::unwrap_used)]
#![feature(
    const_option_ext,
    stmt_expr_attributes,
    box_patterns,
    closure_track_caller,
    error_generic_member_access
)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use indoc::formatdoc;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode, User};
use teloxide::utils::markdown;

use crate::entity::prelude::UserWhitelistStatus;
use crate::state::{AppState, UserState};
use crate::user_service::UserService;

mod entity;
mod logger;
mod lyrics;
mod metrics;
mod profanity;
mod spotify;
mod spotify_auth_service;
mod state;
mod telegram;
mod tick;
mod track_status_service;
mod user_service;
mod utils;
mod whitelist;

async fn sync_name(state: &UserState, tg_user: Option<&User>) -> anyhow::Result<()> {
    let spotify_user = state.spotify_user().await?.map(|spotify_user| {
        spotify_user
            .display_name
            .as_deref()
            .unwrap_or("unknown")
            .to_string()
    });

    let tg_user = tg_user.map(|tg_user| {
        format!(
            "{} {} {}",
            tg_user.first_name,
            tg_user.last_name.as_deref().unwrap_or_default(),
            tg_user
                .username
                .as_deref()
                .map(|username| format!("(@{})", username))
                .unwrap_or_default()
        )
        .trim()
        .to_string()
    });

    let name = vec![tg_user, spotify_user]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" | ");

    UserService::sync_name(state.app.db(), &state.user_id, &name).await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id))]
async fn whitelisted(state: &UserState) -> anyhow::Result<bool> {
    let res = state
        .app
        .whitelist()
        .get_status(state.app.db(), &state.user_id)
        .await?;

    let chat_id = ChatId(state.user_id.parse()?);
    match res {
        (UserWhitelistStatus::Allowed, _) => return Ok(true),
        (UserWhitelistStatus::Denied, _) => {
            tracing::info!("Denied user tried to use bot");

            state
                .app
                .bot()
                .send_message(chat_id, "Sorry, your join request was rejected...")
                .parse_mode(ParseMode::MarkdownV2)
                .send()
                .await?;
        },
        (UserWhitelistStatus::Pending, true) => {
            tracing::info!("New user was sent a request to join");

            let message = formatdoc!(
                "
                    This bot is in whitelist mode\\.
                    Admin already notified that you want to join, but you also can contact [admin](tg://user?id={}) and send this message to him\\.

                    User Id: `{}`",
                state.app.whitelist().contact_admin(),
                state.user_id,
            );

            state
                .app
                .bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::MarkdownV2)
                .send()
                .await?;

            let message = formatdoc!(
                "
                    New [user](tg://user?id={user_id}) wants to join\\!

                    `/whitelist allow {user_id}`
                    `/whitelist deny {user_id}`
                ",
                user_id = state.user_id,
            );

            state
                .app
                .bot()
                .send_message(
                    ChatId(state.app.whitelist().contact_admin().parse()?),
                    message,
                )
                .parse_mode(ParseMode::MarkdownV2)
                .send()
                .await?;
        },
        (UserWhitelistStatus::Pending, false) => {
            tracing::info!("Pending user tried to use bot");

            let message = formatdoc!(
                "
                    This bot is in whitelist mode\\.
                    Your request was already sent, but admin didn't decided yet\\.
                    You can contact [him](tg://user?id={}) to speedup the process\\.
                    Send him this message, this will drastically help\\.

                    User Id: `{}`",
                state.app.whitelist().contact_admin(),
                state.user_id,
            );

            state
                .app
                .bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::MarkdownV2)
                .send()
                .await?;
        },
    };

    Ok(false)
}

async fn run() {
    // profanity::check_cases();

    logger::init().await.expect("Logger should be built");

    tracing::info!(
        git_commit_timestamp = env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        git_sha = env!("VERGEN_GIT_SHA"),
        "Starting Rustify bot..."
    );

    let app_state = AppState::init().await.expect("State to be built");

    tokio::spawn(utils::listen_for_ctrl_c());
    tokio::spawn(tick::check_playing(app_state));
    tokio::spawn(metrics::collect_daemon(app_state));

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(move |m: Message, bot: Bot| async {
                let state = app_state.user_state(&m.chat.id.to_string()).await?;

                if !whitelisted(&state).await? {
                    return Ok(());
                }

                if let Err(err) = sync_name(&state, m.from()).await {
                    tracing::error!(err = ?err, user_id = state.user_id.as_str(), "Failed syncing user name");
                }

                let clone = (m.clone(), bot.clone());

                let result = telegram::handle_message(m, bot, &state).await;

                let (m, bot) = clone;
                if let Err(err) = &result {
                    tracing::error!(err = ?err, "Error on message handling");
                    bot.send_message(
                        m.chat.id,
                        markdown::escape("Sorry, error has happened :("),
                    )
                        .parse_mode(ParseMode::MarkdownV2)
                        .send()
                        .await?;
                }

                result
            }),
        )
        .branch(Update::filter_callback_query().endpoint(
            move |q: CallbackQuery, bot: Bot| async {
                let state = app_state.user_state(&q.from.id.to_string()).await?;

                telegram::inline_buttons::handle(q, bot, &state).await
            },
        ));

    let mut dispatcher = Dispatcher::builder(app_state.bot().clone(), handler).build();

    let token = dispatcher.shutdown_token();

    tokio::spawn(async move {
        utils::ctrl_c().await;

        token.shutdown().expect("To be good").await;
    });

    dispatcher.dispatch().await;
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
