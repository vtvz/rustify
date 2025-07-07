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
use crate::utils::StringUtils;

pub async fn handle_add_word(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
    word: String,
) -> anyhow::Result<HandleStatus> {
    let count_words =
        UserWordWhitelistService::count_ok_words_for_user(app.db(), state.user_id()).await?;

    let validate = |word: &str| {
        if count_words >= 20 {
            return Some(t!(
                "user-word-whitelist.limit-amount",
                locale = state.locale(),
                limit = 20
            ));
        }

        if word.is_empty() {
            return Some(t!(
                "user-word-whitelist.add-provide-word",
                locale = state.locale(),
                command = UserCommandDisplay::AddWhitelistWord,
            ));
        }

        if word.chars_len() > 16 {
            return Some(t!(
                "user-word-whitelist.limit-length",
                locale = state.locale(),
                limit = 16,
            ));
        }

        None
    };

    if let Some(message) = validate(&word) {
        app.bot()
            .send_message(chat_id, message)
            .reply_markup(StartKeyboard::markup(state.locale()))
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
        t!(
            "user-word-whitelist.word-added",
            locale = state.locale(),
            word = word,
            command = UserCommandDisplay::ListWhitelistWords,
        )
    } else {
        t!(
            "user-word-whitelist.word-exist",
            locale = state.locale(),
            word = word,
            command = UserCommandDisplay::ListWhitelistWords,
        )
    };

    app.bot()
        .send_message(chat_id, message)
        .reply_markup(StartKeyboard::markup(state.locale()))
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
        let message = t!(
            "user-word-whitelist.remove-provide-word",
            locale = state.locale(),
            command = UserCommandDisplay::RemoveWhitelistWord,
        );

        app.bot()
            .send_message(chat_id, message)
            .reply_markup(StartKeyboard::markup(state.locale()))
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let removed =
        UserWordWhitelistService::remove_ok_word_for_user(app.db(), state.user_id(), &word).await?;

    let message = if removed {
        t!(
            "user-word-whitelist.removed",
            locale = state.locale(),
            word = word
        )
    } else {
        t!(
            "user-word-whitelist.doesnt-exist",
            locale = state.locale(),
            word = word,
            command = UserCommandDisplay::ListWhitelistWords,
        )
    };

    app.bot()
        .send_message(chat_id, message)
        .reply_markup(StartKeyboard::markup(state.locale()))
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
        t!(
            "user-word-whitelist.empty",
            locale = state.locale(),
            command = UserCommandDisplay::AddWhitelistWord,
        )
    } else {
        let words = words
            .into_iter()
            .map(|word| format!("• <code>{word}</code>"))
            .sorted()
            .collect_vec()
            .join("\n");

        t!(
            "user-word-whitelist.list",
            locale = state.locale(),
            words = words,
            command = UserCommandDisplay::RemoveWhitelistWord,
        )
    };

    app.bot()
        .send_message(chat_id, message.trim())
        .reply_markup(StartKeyboard::markup(state.locale()))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
