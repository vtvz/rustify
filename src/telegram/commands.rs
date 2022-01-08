use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::BotCommand;
use teloxide::utils::command::ParseError;

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
        Command::Dislike => {
            super::helpers::handle_dislike(cx, state).await?;
        }
        Command::Register => {
            super::helpers::handle_register_invite(cx, state).await?;
        }
        Command::Help => {
            cx.answer(Command::descriptions()).send().await?;
        }
    }
    Ok(true)
}
