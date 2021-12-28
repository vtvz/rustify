extern crate derive_more;
#[macro_use]
extern crate serde;

use std::env;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use dotenv::dotenv;
use futures::FutureExt;
use rspotify::model::PlayableItem;
use rspotify::prelude::*;
use rspotify::{clients::OAuthClient, scopes, AuthCodeSpotify, Token};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use teloxide::utils::command::{BotCommand, ParseError};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::commands::Command;
use crate::inline_buttons::InlineButtons;
use crate::keyboards::StartKeyboard;

mod commands;
mod inline_buttons;
mod keyboards;

async fn handle_message_button(
    cx: UpdateWithCx<Bot, CallbackQuery>,
    _state: &AppState,
) -> anyhow::Result<()> {
    let data = cx
        .update
        .data
        .ok_or_else(|| anyhow!("Callback needs data"))?;

    let button: InlineButtons = data.parse()?;

    match button {
        InlineButtons::Cancel(id) => {
            cx.requester
                .answer_callback_query(cx.update.id)
                .text(id)
                .send()
                .await?;

            cx.requester
                .edit_message_text(
                    cx.update.from.id,
                    cx.update.message.unwrap().id,
                    "Dislike cancelled",
                )
                .send()
                .await?;
        }
    }

    Ok(())
}

async fn handle_command(
    cx: &UpdateWithCx<Bot, Message>,
    _state: &AppState,
) -> anyhow::Result<bool> {
    let text = cx.update.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(false);
    }

    let command = Command::parse(text, "Something bot name");

    if let Err(ParseError::UnknownCommand(command)) = command {
        cx.answer(format!(
            "Command `{}` not found: \n\n{}",
            command,
            Command::descriptions()
        ))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

        return Ok(true);
    }

    let command = command?;

    match command {
        Command::Start => {
            cx.answer("Here is your keyboard")
                .reply_markup(StartKeyboard::markup())
                .send()
                .await?;
        }
        Command::Echo(text) => {
            cx.answer(format!("Echo back: {}", text)).send().await?;
        }
        Command::Help => {
            cx.answer(Command::descriptions()).send().await?;
        }
    }
    Ok(true)
}

async fn handle_button(cx: &UpdateWithCx<Bot, Message>, state: &AppState) -> Result<bool> {
    let text = cx.update.text().context("No text available")?;

    let button = StartKeyboard::from_str(text);

    if button.is_err() {
        return Ok(false);
    }

    let button = button?;

    match button {
        StartKeyboard::Dislike => {
            let playing = state.spotify.current_playing(None, None::<&[_]>).await?;
            let playing = match playing {
                Some(playing) => playing.item,
                None => {
                    cx.answer("Nothing is currently playing").send().await?;

                    return Ok(true);
                }
            };

            let item = match playing {
                Some(item) => item,
                None => {
                    cx.answer("Nothing is currently playing").send().await?;

                    return Ok(true);
                }
            };

            let track = match item {
                PlayableItem::Track(item) => item,
                _ => {
                    cx.answer("I don't skip podcasts").send().await?;

                    return Ok(true);
                }
            };

            let track_id = match track.id {
                Some(id) => id.id().to_string(),
                None => {
                    cx.answer("I don't skip local files").send().await?;

                    return Ok(true);
                }
            };

            let artists = track
                .artists
                .iter()
                .map(|art| art.name.as_ref())
                .collect::<Vec<_>>()
                .join(", ");

            #[rustfmt::skip]
            cx.answer(format!("Disliked `{} — {}`", artists, track.name))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
                    vec![
                        vec![InlineKeyboardButton::new(
                            "Cancel ↩",
                            InlineButtons::Cancel(track_id).into(),
                        )]
                    ],
                )))
                .send()
                .await?;
        }
        StartKeyboard::Cleanup => println!("Cleanup"),
        StartKeyboard::Stats => println!("Stats"),
    }

    Ok(true)
}

async fn handle_message(cx: UpdateWithCx<Bot, Message>, state: &AppState) -> Result<()> {
    let _ = handle_command(&cx, state).await? || handle_button(&cx, state).await?;

    Ok(())
}

async fn spotify() -> AuthCodeSpotify {
    let config = rspotify::Config {
        token_refreshing: true,
        ..Default::default()
    };

    let creds = rspotify::Credentials::new(
        env::var("SPOTIFY_ID").unwrap().as_ref(),
        env::var("SPOTIFY_SECRET").unwrap().as_ref(),
    );

    let oauth = rspotify::OAuth {
        redirect_uri: "http://localhost:8080/callback".into(),
        // TODO Reduce to minimum
        scopes: scopes!(
            "ugc-image-upload",
            "user-read-playback-state",
            "user-modify-playback-state",
            "user-read-currently-playing",
            "user-read-private",
            "user-read-email",
            "user-follow-modify",
            "user-follow-read",
            "user-library-modify",
            "user-library-read",
            "app-remote-control",
            "user-read-playback-position",
            "user-top-read",
            "user-read-recently-played",
            "playlist-modify-private",
            "playlist-read-collaborative",
            "playlist-read-private",
            "playlist-modify-public"
        ),
        ..Default::default()
    };

    let spotify = rspotify::AuthCodeSpotify::with_config(creds.clone(), oauth, config.clone());

    *spotify.token.lock().await.unwrap() = Some(Token {
        access_token: env::var("SPOTIFY_ACCESS_TOKEN").unwrap(),
        refresh_token: Some(env::var("SPOTIFY_REFRESH_TOKEN").unwrap()),
        ..Default::default()
    });

    spotify

    /*
    let url = spotify.get_authorize_url(false).unwrap();

    println!("{:?}", url);

    let hello = {
        warp::path("callback")
            .and(warp::query::query::<HashMap<String, String>>())
            .and_then(move |name: HashMap<String, String>| {
                let mut spotify = spotify.clone();
                async move {
                    let code = match name.get("code") {
                        Some(code) => code,
                        None => return Err(warp::reject::not_found()),
                    };

                    spotify.request_token(code);

                    Ok(format!("There is your code: {}", code))
                }
            })
    };

    warp::serve(hello)
        .run(([0, 0, 0, 0], 8080))
        .await;
     */
}

#[derive(Clone)]
struct AppState {
    spotify: AuthCodeSpotify,
}

async fn check_playing(state: &AppState) -> anyhow::Result<()> {
    let _playing = state.spotify.current_playing(None, None::<&[_]>).await?;
    // println!("{:?}", playing);

    Ok(())
}

async fn run() {
    dotenv().ok();

    let spotify = spotify().await;

    teloxide::enable_logging!();
    log::info!("Starting rustify bot...");
    let bot = Bot::new(env::var("TELEGRAM_BOT_TOKEN").unwrap());

    // Make state global static variable to prevent hassle with Arc and cloning this mess
    let state = AppState { spotify };
    let state = Box::new(state);
    let state = &*Box::leak(state);

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            if let Err(err) = check_playing(state).await {
                log::error!("{}: {:?}", "Something gone wrong", err)
            }
        }
    });

    Dispatcher::new(bot)
        .messages_handler({
            move |rx: DispatcherHandlerRx<Bot, Message>| {
                UnboundedReceiverStream::new(rx)
                    .for_each(move |cx| async move {
                        let result = handle_message(cx, state).await;

                        if let Err(err) = result {
                            log::error!("{:?}", err);
                        }
                    })
                    .boxed()
            }
        })
        .callback_queries_handler({
            move |rx: DispatcherHandlerRx<Bot, CallbackQuery>| {
                UnboundedReceiverStream::new(rx)
                    .for_each(move |cx| async {
                        let result = handle_message_button(cx, state).await;

                        if let Err(err) = result {
                            log::error!("{:?}", err);
                        }
                    })
                    .boxed()
            }
        })
        .dispatch()
        .await;
}

#[tokio::main]
async fn main() {
    run().await;
}
