use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::{BotCommands, ParseError};
use teloxide::utils::html;

use super::keyboards::StartKeyboard;
use crate::entity::prelude::*;
use crate::state::{AppState, UserState};
use crate::user_service::UserService;

#[derive(BotCommands, PartialEq, Eq, Debug)]
#[command(rename_rule = "lowercase", parse_with = "split")]
pub enum Command {
    #[command(description = "start")]
    Start,
    #[command(description = "show keyboard")]
    Keyboard,
    #[command(description = "dislike current track")]
    Dislike,
    #[command(description = "delete disliked tracks from your playlists")]
    Cleanup,
    #[command(description = "show details about currently playing track")]
    Details,
    #[command(description = "show statistics about disliked tracks")]
    Stats,
    #[command(description = "login to spotify")]
    Register,
    #[command(description = "show this help")]
    Help,

    #[command(hide)]
    Whitelist(String, String),
}

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<bool> {
    let text = m.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(false);
    }

    let command = match Command::parse(text, "RustifyBot") {
        Err(ParseError::UnknownCommand(command)) => {
            app.bot()
                .send_message(
                    m.chat.id,
                    format!(
                        "Command <code>{}</code> not found: \n\n{}",
                        html::escape(&command),
                        html::escape(&Command::descriptions().to_string())
                    ),
                )
                .parse_mode(ParseMode::Html)
                .send()
                .await?;

            return Ok(true);
        },
        Err(ParseError::IncorrectFormat(_)) => return Ok(false),
        Err(var) => return Err(var.into()),
        Ok(command) => command,
    };

    match command {
        Command::Start | Command::Keyboard => {
            if state.is_spotify_authed().await {
                UserService::set_status(app.db(), state.user_id(), UserStatus::Active).await?;

                app.bot()
                    .send_message(m.chat.id, "Here is your keyboard")
                    .reply_markup(StartKeyboard::markup())
                    .send()
                    .await?;
            } else {
                super::helpers::send_register_invite(app, m.chat.id).await?;
            }
        },
        Command::Dislike => {
            return super::handlers::dislike::handle(app, state, m).await;
        },
        Command::Cleanup => {
            return super::handlers::cleanup::handle(app, state, m).await;
        },
        Command::Stats => return super::handlers::stats::handle(app, state, m).await,
        Command::Details => {
            return super::handlers::details::handle_current(app, state, m).await;
        },
        Command::Register => {
            return super::helpers::send_register_invite(app, m.chat.id).await;
        },
        Command::Help => {
            app.bot()
                .send_message(m.chat.id, Command::descriptions().to_string())
                .send()
                .await?;
        },
        Command::Whitelist(action, user_id) => {
            return super::handlers::whitelist::handle(app, state, m, action, user_id).await;
        },
    }
    Ok(true)
}
