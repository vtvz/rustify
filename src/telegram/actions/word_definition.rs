use itertools::Itertools as _;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::UserLocale;
use crate::services::{WordDefinitionService, WordStatsService};
use crate::telegram::commands_admin::AdminCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons_admin::AdminInlineButtons;
use crate::utils::teloxide::CallbackQueryExt as _;

#[tracing::instrument(skip_all, fields(%locale, %word))]
async fn generate_and_send_definition(
    app: &'static App,
    message: &Message,
    locale: String,
    word: String,
    refresh: bool,
) -> anyhow::Result<()> {
    let Some(ai_config) = app.ai() else {
        app.bot()
            .edit_text(
                message,
                "AI configuration is not available. Word definitions cannot be retrieved.",
            )
            .await?;
        return Ok(());
    };

    app.bot()
        .edit_text(
            message,
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
        AdminInlineButtons::RegenerateWordDefinition {
            locale: locale.clone(),
            word: word.clone(),
        }
        .into_inline_keyboard_button("en"),
    ]]);

    app.bot()
            .edit_text(
                message,
            format!(
                "Definition for <code>{word}</code> (locale: <code>{locale}</code>):\n\n{definition}",
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(%locale, %word))]
pub async fn handle_definition(
    app: &'static App,
    m: &Message,
    locale: String,
    word: String,
    refresh: bool,
) -> anyhow::Result<HandleStatus> {
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

    let message = app.bot().send_message(m.chat.id, "Starting...").await?;

    generate_and_send_definition(app, &message, locale, word, refresh).await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(%locale, %word))]
pub async fn handle_inline_regenerate(
    app: &'static App,
    q: CallbackQuery,
    locale: String,
    word: String,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    generate_and_send_definition(app, &message, locale, word, true).await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(%locale_filter))]
pub async fn handle_list(
    app: &'static App,
    m: &Message,
    locale_filter: String,
) -> anyhow::Result<HandleStatus> {
    send_definitions_page(app, m.chat.id, None, locale_filter, 0).await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(%locale_filter, %page))]
pub async fn handle_inline_list(
    app: &'static App,
    q: CallbackQuery,
    locale_filter: String,
    page: usize,
) -> anyhow::Result<()> {
    app.bot().answer_callback_query(q.id.clone()).await?;

    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    send_definitions_page(app, message.chat.id, Some(message), locale_filter, page).await?;

    Ok(())
}

#[tracing::instrument(skip_all, fields(%locale_filter, %page))]
async fn send_definitions_page(
    app: &'static App,
    chat_id: teloxide::types::ChatId,
    message: Option<Message>,
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
                    locale_codes
                        .iter()
                        .map(|locale| format!(
                            "<code>/{} {locale}</code>",
                            AdminCommandDisplay::ListWordDefinitions
                        ))
                        .join("\n")
                ),
            )
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(());
    }

    let total_items = WordStatsService::count_stats(app.db()).await?;

    if total_items == 0 {
        let text = format!("No word definitions found for locale: <code>{locale_filter}</code>");

        if let Some(message) = &message {
            app.bot()
                .edit_text(message, text)
                .parse_mode(ParseMode::Html)
                .await?;
        } else {
            app.bot()
                .send_message(chat_id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }

        return Ok(());
    }

    let total_pages = total_items.div_ceil(PAGE_SIZE);
    let page = page.min(total_pages.saturating_sub(1));

    let definitions =
        WordStatsService::list_stats_with_definitions(app.db(), &locale_filter, page, PAGE_SIZE)
            .await?;

    let mut lines = vec![];

    lines.push("<b>Word Definitions</b>".to_string());
    lines.push(format!(
        "<i>Page {} of {} ({} total)</i>",
        page + 1,
        total_pages,
        total_items
    ));
    lines.push(String::new());

    lines.push(format!("<b>Locale: {locale_filter}</b>"));
    lines.push(String::new());

    for def in &definitions {
        lines.push(format!(
            "<blockquote>• <code>{}</code>: {}",
            def.word,
            def.definition
                .as_deref()
                .unwrap_or("<i>word have no definition yet</i>")
        ));
        lines.push(String::new());
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
        lines.push(String::new());
    }

    // Create pagination buttons
    let mut buttons = vec![];

    if page > 0 {
        buttons.push(
            AdminInlineButtons::WordDefinitionsPage {
                locale: locale_filter.clone(),
                page: page - 1,
                is_next: false,
            }
            .into_inline_keyboard_button("en"),
        );
    }

    if page < total_pages - 1 {
        buttons.push(
            AdminInlineButtons::WordDefinitionsPage {
                locale: locale_filter.clone(),
                page: page + 1,
                is_next: true,
            }
            .into_inline_keyboard_button("en"),
        );
    }

    let keyboard = if buttons.is_empty() {
        None
    } else {
        Some(InlineKeyboardMarkup::new(vec![buttons]))
    };

    if let Some(message) = &message {
        let mut req = app
            .bot()
            .edit_text(message, lines.join("\n"))
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
