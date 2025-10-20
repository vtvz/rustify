use anyhow::Context;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::UserLocale;
use crate::telegram::commands::AdminCommandDisplay;
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

    let locale_codes = UserLocale::locale_codes();

    if !locale_codes.contains(&locale) {
        app.bot()
            .send_message(
                m.chat.id,
                format!(
                    "Locale <code>{locale}</code> does not exist:\n\n{}",
                    locale_codes.join("\n")
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

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

pub async fn handle_list(
    app: &'static App,
    m: &Message,
    locale_filter: String,
) -> anyhow::Result<HandleStatus> {
    send_definitions_page(app, m.chat.id, None, locale_filter, 0).await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_list_page(
    app: &'static App,
    q: CallbackQuery,
    locale_filter: String,
    page: usize,
) -> anyhow::Result<()> {
    app.bot().answer_callback_query(q.id.clone()).await?;

    let message = q.message.clone().context("Message is empty")?;
    let message_id = message.id();
    let chat_id = message.chat().id;

    send_definitions_page(app, chat_id, Some(message_id), locale_filter, page).await?;

    Ok(())
}

async fn send_definitions_page(
    app: &'static App,
    chat_id: teloxide::types::ChatId,
    message_id: Option<teloxide::types::MessageId>,
    locale_filter: String,
    page: usize,
) -> anyhow::Result<()> {
    const PAGE_SIZE: usize = 10;

    let locale_codes = UserLocale::locale_codes();

    if !locale_codes.contains(&locale_filter) {
        app.bot()
            .send_message(
                chat_id,
                format!(
                    "Locale <code>{locale_filter}</code> does not exist:\n\n{}",
                    locale_codes.join("\n")
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(());
    }

    let total_items =
        WordDefinitionService::count_definitions_with_stats(app.db(), &locale_filter).await?;

    if total_items == 0 {
        let message = format!("No word definitions found for locale: <code>{locale_filter}</code>");

        if let Some(msg_id) = message_id {
            app.bot()
                .edit_message_text(chat_id, msg_id, message)
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            app.bot()
                .send_message(chat_id, message)
                .parse_mode(ParseMode::Html)
                .await?;
        }

        return Ok(());
    }

    let total_pages = total_items.div_ceil(PAGE_SIZE);
    let page = page.min(total_pages.saturating_sub(1));

    // Get paginated results from database
    let definitions = WordDefinitionService::list_definitions_with_stats(
        app.db(),
        &locale_filter,
        page,
        PAGE_SIZE,
    )
    .await?;

    let mut lines = vec![];

    lines.push("<b>Word Definitions</b>".to_string());
    lines.push(format!(
        "<i>Page {} of {} ({} total)</i>",
        page + 1,
        total_pages,
        total_items
    ));
    lines.push("".into());

    lines.push(format!("<b>Locale: {locale_filter}</b>"));
    lines.push("".into());

    for def in &definitions {
        lines.push(format!(
            "<blockquote>• <code>{}</code>: {}",
            def.word, def.definition
        ));
        lines.push("".into());
        lines.push(format!(
            "<code>[ check: {} | details:{} | analyze:{} ]</code>",
            def.check_occurrences, def.details_occurrences, def.analyze_occurrences
        ));
        lines.push(format!(
            "<code>/{} {} {}</code></blockquote>",
            AdminCommandDisplay::ResetWordDefinition,
            locale_filter,
            def.word
        ));
        lines.push("".into());
    }

    // Create pagination buttons
    let mut buttons = vec![];

    if page > 0 {
        buttons.push(
            InlineButtons::WordDefinitionsPage {
                locale: locale_filter.clone(),
                page: page - 1,
            }
            .into_inline_keyboard_button("en"),
        );
    }

    if page < total_pages - 1 {
        buttons.push(
            InlineButtons::WordDefinitionsPage {
                locale: locale_filter.clone(),
                page: page + 1,
            }
            .into_inline_keyboard_button("en"),
        );
    }

    let keyboard = if !buttons.is_empty() {
        Some(InlineKeyboardMarkup::new(vec![buttons]))
    } else {
        None
    };

    if let Some(msg_id) = message_id {
        let mut req = app
            .bot()
            .edit_message_text(chat_id, msg_id, lines.join("\n"))
            .parse_mode(ParseMode::Html);

        if let Some(kb) = keyboard {
            req = req.reply_markup(kb);
        }

        req.await?;
    } else {
        let mut req = app
            .bot()
            .send_message(chat_id, lines.join("\n"))
            .parse_mode(ParseMode::Html);

        if let Some(kb) = keyboard {
            req = req.reply_markup(kb);
        }

        req.await?;
    }

    Ok(())
}
