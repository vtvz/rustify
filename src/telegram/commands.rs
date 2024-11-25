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
    app_state: &'static AppState,
    state: &UserState,
    bot: &Bot,
    m: &Message,
) -> anyhow::Result<bool> {
    let text = m.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(false);
    }

    let command = Command::parse(text, "Something bot name");

    if let Err(ParseError::UnknownCommand(command)) = command {
        bot.send_message(
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
    }

    let command = command?;

    match command {
        Command::Start | Command::Keyboard => {
            if state.is_spotify_authed().await {
                UserService::set_status(app_state.db(), state.user_id(), UserStatus::Active)
                    .await?;

                bot.send_message(m.chat.id, "Here is your keyboard")
                    .reply_markup(StartKeyboard::markup())
                    .send()
                    .await?;
            } else {
                super::helpers::send_register_invite(app_state, bot, m.chat.id).await?;
            }
        },
        Command::Dislike => {
            return super::handlers::dislike::handle(app_state, state, bot, m).await;
        },
        Command::Cleanup => {
            return super::handlers::cleanup::handle(app_state, state, bot, m).await;
        },
        Command::Stats => return super::handlers::stats::handle(app_state, state, bot, m).await,
        Command::Details => {
            return super::handlers::details::handle_current(app_state, state, bot, m).await;
        },
        Command::Register => {
            return super::helpers::send_register_invite(app_state, bot, m.chat.id).await;
        },
        Command::Help => {
            bot.send_message(m.chat.id, Command::descriptions().to_string())
                .send()
                .await?;
        },
        Command::Whitelist(action, user_id) => {
            return super::handlers::whitelist::handle(app_state, state, bot, m, action, user_id)
                .await;
        },
    }
    Ok(true)
}
