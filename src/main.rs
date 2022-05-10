#![warn(clippy::unwrap_used)]
#![feature(
    option_result_contains,
    stmt_expr_attributes,
    let_else,
    backtrace,
    box_patterns
)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use teloxide::prelude::*;
use teloxide::types::{ParseMode, User};
use teloxide::utils::markdown;

use crate::errors::GenericResult;
use crate::state::{AppState, UserState};
use crate::user_service::UserService;

mod entity;
mod errors;
mod logger;
mod lyrics;
mod metrics;
mod profanity;
mod rickroll;
mod spotify;
mod spotify_auth_service;
mod state;
mod telegram;
mod tick;
mod track_status_service;
mod user_service;
mod utils;

async fn sync_name(state: &UserState, tg_user: Option<&User>) -> GenericResult<()> {
    let spotify_user = state.spotify_user.as_ref().map(|spotify_user| {
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

    UserService::sync_name(&state.app.db, &state.user_id, &name).await?;

    Ok(())
}

async fn run() {
    // profanity::check_cases();

    logger::init().await.expect("Logger should be built");

    tracing::info!(
        build_timestamp = env!("VERGEN_BUILD_TIMESTAMP"),
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

                if let Err(err) = sync_name(&state, m.from()).await {
                    let err = err.anyhow();
                    tracing::error!(err = ?err, user_id = state.user_id.as_str(), "Failed syncing user name: {:?}", err);
                }

                let clone = (m.clone(), bot.clone());

                let result = telegram::handle_message(m, bot, &state).await;

                let (m, bot) = clone;
                if let Err(err) = &result {
                    let err = err;
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

    let mut dispatcher = Dispatcher::builder(app_state.bot.clone(), handler).build();

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
