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
mod genius;
mod profanity;
mod rickroll;
mod spotify;
mod spotify_auth_service;
mod state;
mod telegram;
mod tick;
mod track_status_service;

async fn run() {
    let app_state = AppState::init().await.expect("State to be built");
    tokio::spawn(tick::check_playing(app_state));

    log::info!("Starting rustify bot...");

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
