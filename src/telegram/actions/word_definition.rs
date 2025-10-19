use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::word_definition_service::WordDefinitionService;

pub async fn handle(
    app: &'static App,
    m: &Message,
    locale: String,
    word: String,
    refresh: bool,
) -> anyhow::Result<HandleStatus> {
    let Some(ai_config) = app.ai() else {
        app.bot()
            .send_message(
                m.chat.id,
                "AI configuration is not available. Word definitions cannot be retrieved.",
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let generating_msg = app
        .bot()
        .send_message(
            m.chat.id,
            format!(
                "Generating definition for <code>{word}</code> (locale: <code>{locale}</code>)...",
            ),
        )
        .parse_mode(ParseMode::Html)
        .await?;

    if refresh {
        WordDefinitionService::clear_definition(app.db(), &locale, &word).await?;
    }

    let definition =
        WordDefinitionService::get_definition(app.db(), &locale, ai_config, &word).await?;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineButtons::RegenerateWordDefinition {
            locale: locale.clone(),
            word: word.clone(),
        }
        .into_inline_keyboard_button("en"),
    ]]);

    // Edit the message with the actual definition
    app.bot()
        .edit_message_text(
            generating_msg.chat.id,
            generating_msg.id,
            format!(
                "Definition for <code>{word}</code> (locale: <code>{locale}</code>):\n\n{definition}",
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_inline(
    app: &'static App,
    q: CallbackQuery,
    locale: String,
    word: String,
) -> anyhow::Result<()> {
    let chat_id = q.from.id;

    let message_id = q.message.clone().context("Message is empty")?.id();

    let Some(ai_config) = app.ai() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("AI configuration is not available")
            .await?;
        return Ok(());
    };

    // Answer the callback query immediately
    app.bot().answer_callback_query(q.id.clone()).await?;

    // Edit message to show "generating" status
    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            format!(
                "Generating definition for <code>{word}</code> (locale: <code>{locale}</code>)...",
            ),
        )
        .parse_mode(ParseMode::Html)
        .await?;

    WordDefinitionService::clear_definition(app.db(), &locale, &word).await?;

    let definition =
        WordDefinitionService::get_definition(app.db(), &locale, ai_config, &word).await?;

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineButtons::RegenerateWordDefinition {
            locale: locale.clone(),
            word: word.clone(),
        }
        .into_inline_keyboard_button("en"),
    ]]);

    app.bot()
        .edit_message_text(
            chat_id,
            message_id,
            format!(
                "Definition for <code>{word}</code> (locale: <code>{locale}</code>):\n\n{definition}",
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}
