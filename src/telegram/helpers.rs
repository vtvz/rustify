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
use teloxide::utils::command::{BotCommand, ParseError};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing_subscriber::fmt::format::Full;

use crate::entity::prelude::TrackStatus;
use crate::spotify::CurrentlyPlaying;
use crate::telegram::commands::Command;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::keyboards::StartKeyboard;
use crate::track_status_service::Status;
use crate::CurrentlyPlaying::Error;

use crate::spotify;
use crate::state::UserState;
use crate::track_status_service;
use crate::USER_ID;

pub async fn handle_dislike(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState<'static>,
) -> Result<bool> {
    let track = match spotify::currently_playing(&state.spotify).await {
        Error(error) => return Err(error),
        CurrentlyPlaying::None(_) => return Ok(true),
        CurrentlyPlaying::Ok(track) => track,
    };

    let track_id = spotify::get_track_id(&track);

    track_status_service::set_status(
        &state.app.db,
        USER_ID.to_string(),
        track_id.clone(),
        track_status_service::Status::Disliked,
    )
    .await?;

    cx.answer(format!("Disliked {}", spotify::create_track_name(&track)))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Cancel(track_id).into()]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}
