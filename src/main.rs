#![warn(clippy::unwrap_used)]
#![allow(unused_imports)]
#![feature(option_result_contains, stmt_expr_attributes, let_else)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use std::collections::HashMap;
use std::time::Duration;

use futures::FutureExt;
use rspotify::clients::OAuthClient;
use rspotify::model::{FullTrack, TrackId};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::spotify::CurrentlyPlaying;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state::AppState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::{Status, TrackStatusService};

mod entity;
mod genius;
mod spotify;
mod spotify_auth_service;
mod state;
mod telegram;
mod track_status_service;

async fn check_bad_words(state: &state::UserState, track: &FullTrack) -> anyhow::Result<()> {
    let Some(hit) = genius::search_for_track(state, track).await? else {
        return Ok(())
    };

    let lyrics = genius::get_lyrics(&hit.result.url).await?;

    let bad_lines: Vec<_> = genius::find_bad_words(lyrics);

    if bad_lines.is_empty() {
        return Ok(());
    }

    let mut lines = bad_lines.len();
    let message = loop {
        let message = format!(
            // TODO Return spoilers after teloxide update
            // "has bad words: \n ||{}||",
            "Current song \\({}\\) probably has bad words \\(ignore in case of false positive\\): \n\n{}\n\n[Genius Source]({})",
            spotify::create_track_name(track),
            bad_lines[0..lines].join("\n"),
            hit.result.url
        );

        if message.len() <= 4096 {
            break message;
        }

        lines -= 1;
    };

    state
        .app
        .bot
        .send_message(state.user_id.clone(), message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(spotify::get_track_id(track)).into()],
                vec![InlineButtons::Ignore(spotify::get_track_id(track)).into()],
            ],
        )))
        .send()
        .await?;

    Ok(())
}

async fn check_playing(app_state: &'static state::AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    let mut prevs: HashMap<String, TrackId> = HashMap::new();
    loop {
        interval.tick().await;

        let user_ids = match SpotifyAuthService::get_registered(&app_state.db).await {
            Ok(user_ids) => user_ids,
            Err(err) => {
                log::error!("{:?}", err);
                continue;
            }
        };

        for user_id in user_ids {
            let state = match app_state.user_state(user_id.clone()).await {
                Ok(state) => state,
                Err(err) => {
                    log::error!("{}: {:?}", "Something went wrong", err);
                    continue;
                }
            };
            let playing = spotify::currently_playing(&*state.spotify.read().await).await;

            let track = match playing {
                CurrentlyPlaying::Err(err) => {
                    log::error!("{}: {:?}", "Something went wrong", err);
                    continue;
                }
                CurrentlyPlaying::None(_message) => {
                    continue;
                }
                CurrentlyPlaying::Ok(track) => track,
            };

            let status = TrackStatusService::get_status(
                &state.app.db,
                state.user_id.clone(),
                spotify::get_track_id(&track),
            )
            .await;

            match status {
                Status::Disliked => {
                    if let Err(err) = state.spotify.read().await.next_track(None).await {
                        log::error!("{}: {:?}", "Something went wrong", err)
                    }
                }
                Status::None => {
                    if prevs.get(&user_id) == track.id.as_ref() {
                        continue;
                    }

                    if let Err(err) = check_bad_words(&state, &track).await {
                        log::error!("{}: {:?}", "Something went wrong", err)
                    }
                }
                Status::Ignore => {}
            }

            match track.id {
                Some(id) => prevs.insert(user_id, id),
                None => None,
            };
        }
    }
}

async fn run() {
    let app_state = AppState::init().await.expect("State to be built");
    tokio::spawn(check_playing(app_state));
    match tracing_subscriber::fmt::try_init() {
        Ok(_) => println!("tracing_subscriber::fmt::try_init success"),
        Err(err) => println!("tracing_subscriber::fmt::try_init error: {:?}", err),
    }

    log::info!("Starting rustify bot...");

    Dispatcher::new(app_state.bot.clone())
        .messages_handler(move |rx: DispatcherHandlerRx<Bot, Message>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async move {
                    let state = match app_state.user_state(cx.update.chat_id().to_string()).await {
                        Ok(state) => state,
                        Err(err) => {
                            log::error!("{:?}", err);
                            return;
                        }
                    };

                    let result = telegram::handle_message(&cx, &state).await;

                    if let Err(err) = result {
                        log::error!("{:?}", err);
                    }
                })
                .boxed()
        })
        .callback_queries_handler(move |rx: DispatcherHandlerRx<Bot, CallbackQuery>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async {
                    let state = match app_state.user_state(cx.update.from.id.to_string()).await {
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
