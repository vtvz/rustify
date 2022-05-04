use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::{BotCommands, ParseError};
use teloxide::utils::markdown;

use super::keyboards::StartKeyboard;
use crate::entity::prelude::*;
use crate::errors::{Context, GenericResult};
use crate::rickroll;
use crate::state::UserState;
use crate::user_service::UserService;

#[derive(BotCommands, PartialEq, Debug)]
#[command(rename = "lowercase")]
pub enum Command {
    #[command(description = "start")]
    Start,
    #[command(description = "show keyboard")]
    Keyboard,
    #[command(description = "echo back the message")]
    Echo(String),
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

    #[command(description = "off")]
    Rickroll(String),
}

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> GenericResult<bool> {
    let text = m.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(false);
    }

    let command = Command::parse(text, "Something bot name");

    if let Err(ParseError::UnknownCommand(command)) = command {
        bot.send_message(
            m.chat.id,
            format!(
                "Command `{}` not found: \n\n{}",
                markdown::escape(&command),
                markdown::escape(&Command::descriptions().to_string())
            ),
        )
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

        return Ok(true);
    }

    let command = command?;

    match command {
        Command::Start | Command::Keyboard => {
            if state.is_spotify_authed().await {
                UserService::set_status(&state.app.db, &state.user_id, UserStatus::Active).await?;

                bot.send_message(m.chat.id, "Here is your keyboard")
                    .reply_markup(StartKeyboard::markup())
                    .send()
                    .await?;
            } else {
                super::helpers::send_register_invite(m, bot, state).await?;
            }
        },
        Command::Echo(text) => {
            bot.send_message(m.chat.id, format!("Echo back: {}", text))
                .send()
                .await?;
        },
        Command::Dislike => return super::handlers::dislike::handle(m, bot, state).await,
        Command::Cleanup => return super::handlers::cleanup::handle(m, bot, state).await,
        Command::Stats => return super::handlers::stats::handle(m, bot, state).await,
        Command::Details => return super::handlers::details::handle_current(m, bot, state).await,
        Command::Register => return super::helpers::send_register_invite(m, bot, state).await,
        Command::Help => {
            bot.send_message(m.chat.id, Command::descriptions().to_string())
                .send()
                .await?;
        },
        Command::Rickroll(user_id) => {
            if rickroll::is_admin(&state.user_id) {
                let state = state.app.user_state(&user_id).await?;

                rickroll::queue(&state).await;
            }
        },
    }
    Ok(true)
}
