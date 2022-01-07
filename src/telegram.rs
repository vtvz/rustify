pub mod commands;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;

use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
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

pub async fn handle_message(
    cx: UpdateWithCx<Bot, Message>,
    state: &UserState<'static>,
) -> Result<()> {
    let _ = commands::handle(&cx, state).await? || keyboards::handle(&cx, state).await?;

    Ok(())
}
