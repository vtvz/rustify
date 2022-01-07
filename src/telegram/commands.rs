use teloxide::utils::command::BotCommand;

use std::env;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use censor::{Censor, Custom, Sex, Standard};
use dotenv::dotenv;
use futures::{FutureExt, TryFutureExt};
use genius_rs::Genius;
use rspotify::model::{FullTrack, TrackId};
use rspotify::prelude::*;
use rspotify::{clients::OAuthClient, AuthCodeSpotify};
use sea_orm::prelude::*;
use sea_orm::IntoActiveModel;
use sea_orm::{Database, DatabaseConnection, DbConn, NotSet, Set};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use teloxide::utils::command::ParseError;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing_subscriber::fmt::format::Full;

use crate::entity::prelude::TrackStatus;
use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::keyboards::StartKeyboard;
use crate::track_status_service::Status;
use crate::CurrentlyPlaying::Error;

#[derive(BotCommand, PartialEq, Debug)]
#[command(rename = "lowercase")]
pub enum Command {
    #[command(description = "start")]
    Start,
    #[command(description = "echo back the message")]
    Echo(String),
    #[command(description = "dislike current track")]
    Dislike,
    #[command(description = "show this help")]
    Help,
}

pub async fn handle(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState<'static>,
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
        Command::Dislike => {
            super::helpers::handle_dislike(cx, state).await?;
        }
        Command::Help => {
            cx.answer(Command::descriptions()).send().await?;
        }
    }
    Ok(true)
}
