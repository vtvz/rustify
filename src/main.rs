#![warn(clippy::unwrap_used)]
#![feature(option_result_contains, stmt_expr_attributes, let_else)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use teloxide::prelude2::*;
use teloxide::types::ParseMode;
use teloxide::utils::markdown;

use crate::state::AppState;

mod entity;
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
mod utils;

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

    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();

        *app_state.shutting_down.lock().await = true;
    });

    tokio::spawn(tick::check_playing(app_state));
    tokio::spawn(metrics::collect_daemon(app_state));

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(move |m: Message, bot: Bot| async {
                let state = app_state.user_state(&m.chat.id.to_string()).await?;

                let clone = (m.clone(), bot.clone());

                let result = telegram::handle_message(m, bot, &state).await;

                let (m, bot) = clone;
                if let Err(err) = &result {
                    log::error!("{:?}", err);
                    bot.send_message(
                        m.chat.id,
                        format!(
                            "Sorry, error has happened :\\(\n`{}`",
                            markdown::escape(&format!("{:?}", err))
                        ),
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

    Dispatcher::builder(app_state.bot.clone(), handler)
        .build()
        .setup_ctrlc_handler()
        .dispatch()
        .await;
}

#[tokio::main(worker_threads = 4)]
async fn main() {
    run().await;
}
