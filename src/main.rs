#![allow(unused_imports)]
#![feature(option_result_contains, stmt_expr_attributes)]

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde;

use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::spotify::CurrentlyPlaying;
use crate::state::AppState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::Status;
use anyhow::{Context, Result};
use censor::{Censor, Sex, Standard};
use dotenv::dotenv;
use futures::{FutureExt, TryFutureExt};
use genius_rs::Genius;
use rspotify::clients::OAuthClient;
use rspotify::model::{FullTrack, TrackId};
use rspotify::prelude::*;
use sea_orm::prelude::*;
use sea_orm::IntoActiveModel;
use sea_orm::{Database, DbConn};
use sqlx::migrate::MigrateDatabase;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use teloxide::utils::command::BotCommand;
use tokio_stream::wrappers::UnboundedReceiverStream;

mod entity;
mod genius;
mod spotify;
mod state;
mod telegram;
mod track_status_service;

const USER_ID: &str = "228571569";

async fn check_bad_words(
    state: &state::UserState<'static>,
    track: &FullTrack,
) -> anyhow::Result<()> {
    let artists = track
        .artists
        .iter()
        .map(|art| art.name.as_ref())
        .collect::<Vec<_>>()
        .join(" ");

    let q = format!("{} {}", artists, track.name);

    let hits = state.app.genius.search(q.as_ref()).await?;

    let first = match hits.get(0) {
        None => return Ok(()),
        Some(hit) => hit,
    };

    let lyrics = genius::get_lyrics(&first.result.url).await?;

    let bad_lines: Vec<_> = genius::find_bad_words(lyrics, &state.app.censor);

    if bad_lines.is_empty() {
        return Ok(());
    }

    let message = format!(
        // TODO Return spoilers after teloxide update
        // "has bad words: \n ||{}||",
        "Current song \\({}\\) has bad words: \n\n{}\n\n[Genius Source]({})",
        spotify::create_track_name(track),
        bad_lines.join("\n"),
        first.result.url
    );

    state
        .app
        .bot
        .send_message(USER_ID.to_string(), message)
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
    let state = app_state.user_state(USER_ID.to_string()).await;

    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut prev: Option<TrackId> = None;
    loop {
        interval.tick().await;
        let playing = spotify::currently_playing(&state.spotify).await;

        let track = match playing {
            CurrentlyPlaying::Error(error) => {
                log::error!("{}: {:?}", "Something gone wrong", error);
                continue;
            }
            CurrentlyPlaying::None(_message) => {
                continue;
            }
            CurrentlyPlaying::Ok(track) => track,
        };

        let status = track_status_service::get_status(
            &state.app.db,
            USER_ID.to_string(),
            spotify::get_track_id(&track),
        )
        .await;

        match status {
            Status::Disliked => {
                if let Err(err) = state.spotify.next_track(None).await {
                    log::error!("{}: {:?}", "Something gone wrong", err)
                }
            }
            Status::None => {
                if prev == track.id {
                    continue;
                }

                if let Err(err) = check_bad_words(&state, &track).await {
                    log::error!("{}: {:?}", "Something gone wrong", err)
                }
            }
            Status::Ignore => {}
        }

        prev = track.id;
    }
}

async fn run() {
    let app_state = AppState::init().await.expect("State to be built");
    tokio::spawn(check_playing(app_state));

    Dispatcher::new(app_state.bot.clone())
        .messages_handler(move |rx: DispatcherHandlerRx<Bot, Message>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async move {
                    let state = app_state.user_state(cx.update.chat_id().to_string()).await;
                    let result = telegram::handle_message(cx, &state).await;

                    if let Err(err) = result {
                        log::error!("{:?}", err);
                    }
                })
                .boxed()
        })
        .callback_queries_handler(move |rx: DispatcherHandlerRx<Bot, CallbackQuery>| {
            UnboundedReceiverStream::new(rx)
                .for_each(move |cx| async {
                    let state = app_state.user_state(cx.update.from.id.to_string()).await;

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
