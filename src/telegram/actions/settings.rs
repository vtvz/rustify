use sea_orm::{ActiveModelTrait as _, IntoActiveModel as _, Set};
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;
use crate::user_service::UserService;

pub async fn handle_toggle_profanity_check(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let new_status = !user.cfg_check_profanity;

    let mut user_model = user.into_active_model();

    user_model.cfg_check_profanity = Set(new_status);
    user_model.save(app.db()).await?;

    let message = if new_status {
        "Tracks will be checked on profanity content"
    } else {
        "Tracks won't be checked on profanity content"
    };

    app.bot()
        .send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_toggle_skip_tracks(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let new_status = !user.cfg_skip_tracks;

    let mut user_model = user.into_active_model();

    user_model.cfg_skip_tracks = Set(new_status);
    user_model.save(app.db()).await?;

    let message = if new_status {
        "Disliked tracks will be skipped"
    } else {
        "Disliked tracks won't be skipped"
    };

    app.bot()
        .send_message(chat_id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn handle_set_analysis_language(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
    language: String,
) -> anyhow::Result<HandleStatus> {
    let validate = |language: &str| {
        if language.is_empty() {
            return Some(format!(
                "Provide word <code>/{command} yourword</code>",
                command = UserCommandDisplay::SetAnalysisLanguage,
            ));
        }

        if language.len() > 15 {
            return Some("Language name should be shorter".to_string());
        }

        None
    };

    if let Some(message) = validate(&language) {
        app.bot()
            .send_message(chat_id, message)
            .reply_markup(StartKeyboard::markup())
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let mut user = UserService::obtain_by_id(app.db(), state.user_id())
        .await?
        .into_active_model();

    user.cfg_analysis_language = Set(Some(language.clone()));
    user.save(app.db()).await?;

    let message = format!("Now result of song analysis will be in <b>{language}</b> language");

    app.bot()
        .send_message(chat_id, message)
        .reply_markup(StartKeyboard::markup())
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
