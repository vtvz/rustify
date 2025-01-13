use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::utils::command::{BotCommands, ParseError};

use super::HandleStatus;
use crate::app::App;
use crate::entity::prelude::*;
use crate::telegram::actions;
use crate::telegram::commands::Command;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;
use crate::user_service::UserService;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let text = m.text().context("No text available")?;

    if !text.starts_with('/') {
        return Ok(HandleStatus::Skipped);
    }

    let command = match Command::parse(text, "RustifyBot") {
        Err(ParseError::UnknownCommand(command)) => {
            app.bot()
                .send_message(
                    m.chat.id,
                    Command::descriptions()
                        .global_description(&format!("Command <code>{command}</code> not found.\n\nThere are commands available to you:"))
                        .to_string(),
                )
                .reply_markup(StartKeyboard::markup())
                .parse_mode(ParseMode::Html)
                .await?;

            return Ok(HandleStatus::Handled);
        },
        Err(ParseError::IncorrectFormat(_)) => return Ok(HandleStatus::Skipped),
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
                    .await?;
            } else {
                actions::register::send_register_invite(app, m.chat.id).await?;
            }
        },
        Command::Dislike => {
            return actions::dislike::handle(app, state, m).await;
        },
        Command::Like => {
            return actions::like::handle(app, state, m).await;
        },
        Command::Cleanup => {
            return actions::cleanup::handle(app, state, m).await;
        },
        Command::Stats => return actions::stats::handle(app, state, m).await,
        Command::Details => {
            return actions::details::handle_current(app, state, m).await;
        },
        Command::Register => {
            return actions::register::send_register_invite(app, m.chat.id).await;
        },
        Command::Help => {
            app.bot()
                .send_message(
                    m.chat.id,
                    Command::descriptions()
                        .global_description("Commands available to you")
                        .to_string(),
                )
                .reply_markup(StartKeyboard::markup())
                .await?;
        },
        Command::Whitelist(action, user_id) => {
            return actions::whitelist::handle(app, state, m, action, user_id).await;
        },
        Command::ToggleTrackSkip => {
            return actions::settings::handle_toggle_skip_tracks(app, state, m.chat.id).await;
        },
        Command::ToggleProfanityCheck => {
            return actions::settings::handle_toggle_profanity_check(app, state, m.chat.id).await;
        },
        Command::SetAnalysisLanguage { language } => {
            return actions::settings::handle_set_analysis_language(
                app, state, m.chat.id, language,
            )
            .await;
        },
        Command::AddWhitelistWord { word } => {
            return actions::user_word_whitelist::handle_add_word(app, state, m.chat.id, word)
                .await;
        },
        Command::RemoveWhitelistWord { word } => {
            return actions::user_word_whitelist::handle_remove_word(app, state, m.chat.id, word)
                .await;
        },
        Command::ListWhitelistWords => {
            return actions::user_word_whitelist::handle_list_words(app, state, m.chat.id).await;
        },
    }
    Ok(HandleStatus::Handled)
}
