use anyhow::Context;
use teloxide::prelude::*;
use teloxide::utils::command::{BotCommands, ParseError};

use super::HandleStatus;
use crate::app::App;
use crate::telegram::actions;
use crate::telegram::commands::AdminCommand;
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let text = m.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(HandleStatus::Skipped);
    }

    let command = match AdminCommand::parse(text, "RustifyBot") {
        Err(ParseError::UnknownCommand(_)) => return Ok(HandleStatus::Skipped),
        Err(ParseError::IncorrectFormat(_)) => return Ok(HandleStatus::Skipped),
        Err(var) => return Err(var.into()),
        Ok(command) => command,
    };

    match command {
        AdminCommand::Admin => {
            app.bot()
                .send_message(
                    m.chat.id,
                    AdminCommand::descriptions()
                        .global_description("Admin Commands available to you")
                        .to_string(),
                )
                .await?;
        },
        AdminCommand::Whitelist(action, user_id) => {
            return actions::whitelist::handle(app, m, action, user_id).await;
        },
        AdminCommand::GlobalStats => {
            return actions::global_stats::handle(app, state, m).await;
        },
        AdminCommand::GetAnalyzePrompt => {
            app.bot()
                .send_message(
                    m.chat.id,
                    app.analyze()
                        .map(|config| config.prompt())
                        .unwrap_or("Analyze did not configured"),
                )
                .await?;
        },
    }

    Ok(HandleStatus::Handled)
}
