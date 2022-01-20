use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommand;
use teloxide::utils::command::ParseError;
use teloxide::utils::markdown::escape;

use crate::state::UserState;
use crate::telegram::keyboards::StartKeyboard;

#[derive(BotCommand, PartialEq, Debug)]
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
}

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> anyhow::Result<bool> {
    let text = cx.update.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(false);
    }

    let command = Command::parse(text, "Something bot name");

    if let Err(ParseError::UnknownCommand(command)) = command {
        cx.answer(format!(
            "Command `{}` not found: \n\n{}",
            escape(&command),
            escape(&Command::descriptions())
        ))
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

        return Ok(true);
    }

    let command = command?;

    match command {
        Command::Start | Command::Keyboard => {
            if state.is_spotify_authed().await {
                cx.answer("Here is your keyboard")
                    .reply_markup(StartKeyboard::markup())
                    .send()
                    .await?;
            } else {
                super::helpers::handle_register_invite(cx, state).await?;
            }
        }
        Command::Echo(text) => {
            cx.answer(format!("Echo back: {}", text)).send().await?;
        }
        Command::Dislike => return super::handlers::dislike::handle(cx, state).await,
        Command::Cleanup => return super::handlers::cleanup::handle(cx, state).await,
        Command::Stats => return super::handlers::stats::handle(cx, state).await,
        Command::Details => return super::handlers::details::handle(cx, state).await,
        Command::Register => return super::helpers::handle_register_invite(cx, state).await,
        Command::Help => {
            cx.answer(Command::descriptions()).send().await?;
        }
    }
    Ok(true)
}
