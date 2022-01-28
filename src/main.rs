#![warn(clippy::unwrap_used)]
#![feature(option_result_contains, stmt_expr_attributes, let_else)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use futures::FutureExt;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::markdown;
use tokio_stream::wrappers::UnboundedReceiverStream;

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

    Dispatcher::new(app_state.bot.clone())
        .messages_handler(move |rx: DispatcherHandlerRx<Bot, Message>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async move {
                    let state = match app_state.user_state(&cx.update.chat_id().to_string()).await {
                        Ok(state) => state,
                        Err(err) => {
                            log::error!("{:?}", err);
                            return;
                        }
                    };

                    let result = telegram::handle_message(&cx, &state).await;

                    if let Err(err) = result {
                        log::error!("{:?}", err);
                        cx.answer(format!(
                            "Sorry, error has happened :\\(\n`{}`",
                            markdown::escape(&format!("{:?}", err))
                        ))
                        .parse_mode(ParseMode::MarkdownV2)
                        .send()
                        .await
                        .ok();
                    }
                })
                .boxed()
        })
        .callback_queries_handler(move |rx: DispatcherHandlerRx<Bot, CallbackQuery>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async {
                    let state = match app_state.user_state(&cx.update.from.id.to_string()).await {
                        Ok(state) => state,
                        Err(err) => {
                            log::error!("{:?}", err);
                            return;
                        }
                    };

                    let result = telegram::inline_buttons::handle(cx, &state).await;

                    if let Err(err) = result {
                        log::error!("{:?}", err);
                    }
                })
                .boxed()
        })
        .dispatch()
        .await;
}

#[tokio::main]
async fn main() {
    run().await;
}
