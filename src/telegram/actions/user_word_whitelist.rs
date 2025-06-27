use indoc::formatdoc;
use itertools::Itertools as _;
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;
use crate::user_word_whitelist_service::UserWordWhitelistService;

pub async fn handle_add_word(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
    word: String,
) -> anyhow::Result<HandleStatus> {
    if word.is_empty() {
        let message = format!(
            "Provide word <code>/{command} yourword</code>",
            command = UserCommandDisplay::AddWhitelistWord,
        );

        app.bot()
            .send_message(chat_id, message)
            .reply_markup(StartKeyboard::markup())
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let added = UserWordWhitelistService::add_ok_word_for_user(
        app.db(),
        state.user_id().to_string(),
        word.clone(),
    )
    .await?;

    let message = if added {
        formatdoc!(
            "
                Word <code>'{word}'</code> added to whitelist

                To list all word in whitelist /{command}
            ",
            command = UserCommandDisplay::ListWhitelistWords,
        )
    } else {
        formatdoc!(
            "
                Word <code>'{word}'</code> already in whitelist

                To list all word in whitelist /{command}
            ",
            command = UserCommandDisplay::ListWhitelistWords,
        )
    };

    app.bot()
        .send_message(chat_id, message)
        .reply_markup(StartKeyboard::markup())
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_remove_word(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
    word: String,
) -> anyhow::Result<HandleStatus> {
    if word.is_empty() {
        let message = format!(
            "Provide word <code>/{command} yourword</code>",
            command = UserCommandDisplay::AddWhitelistWord,
        );

        app.bot()
            .send_message(chat_id, message)
            .reply_markup(StartKeyboard::markup())
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let removed =
        UserWordWhitelistService::remove_ok_word_for_user(app.db(), state.user_id(), &word).await?;

    let message = if removed {
        format!("Word <code>'{word}'</code> removed from whitelist")
    } else {
        formatdoc!(
            "
                Word <code>'{word}'</code> not in whitelist
                To list all word in whitelist /{command}
            ",
            command = UserCommandDisplay::ListWhitelistWords,
        )
    };

    app.bot()
        .send_message(chat_id, message)
        .reply_markup(StartKeyboard::markup())
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_list_words(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let words = UserWordWhitelistService::get_ok_words_for_user(app.db(), state.user_id()).await?;

    let message = if words.is_empty() {
        formatdoc!(
            "
                Your whitelist is empty

                Add new word with <code>/{command} your-word</code>
            ",
            command = UserCommandDisplay::AddWhitelistWord,
        )
    } else {
        let words = words
            .into_iter()
            .map(|word| format!("• <code>{word}</code>"))
            .collect_vec()
            .join("\n");

        formatdoc!(
            "
                Words you added to whitelist:

                {words}

                You can remove word with <code>/{command} your-word</code> command
            ",
            command = UserCommandDisplay::RemoveWhitelistWord,
        )
    };

    app.bot()
        .send_message(chat_id, message.trim())
        .reply_markup(StartKeyboard::markup())
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
