use indoc::formatdoc;
use itertools::Itertools as _;
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate::state::{AppState, UserState};
use crate::telegram::handlers::HandleStatus;
use crate::user_word_whitelist_service::UserWordWhitelistService;

pub async fn handle_add_word(
    app: &AppState,
    state: &UserState,
    chat_id: ChatId,
    word: String,
) -> anyhow::Result<HandleStatus> {
    if word.is_empty() {
        let message = "Provide word <code>/add_whitelist_word yourword</code>";

        app.bot()
            .send_message(chat_id, message)
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
        format!("Word <code>'{word}'</code> added to whitelist")
    } else {
        format!("Word <code>'{word}'</code> already in whitelist")
    };

    app.bot()
        .send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_remove_word(
    app: &AppState,
    state: &UserState,
    chat_id: ChatId,
    word: String,
) -> anyhow::Result<HandleStatus> {
    if word.is_empty() {
        let message = "Provide word <code>/remove_whitelist_word yourword</code>";

        app.bot()
            .send_message(chat_id, message)
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let removed =
        UserWordWhitelistService::remove_ok_word_for_user(app.db(), state.user_id(), &word).await?;

    let message = if removed {
        format!("Word <code>'{word}'</code> removed from whitelist")
    } else {
        format!("Word <code>'{word}'</code> was not added")
    };

    app.bot()
        .send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_list_words(
    app: &AppState,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let words = UserWordWhitelistService::get_ok_words_for_user(app.db(), state.user_id()).await?;

    let message = if words.is_empty() {
        formatdoc!(
            "
                Your whitelist is empty
                Add new word with <code>/add_whitelist_word your-word</code>
            "
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

                You can remove word with <code>/remove_whitelist_word your-word</code> command
            "
        )
    };

    app.bot()
        .send_message(chat_id, message.trim())
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
