use sea_orm::{ActiveModelTrait as _, IntoActiveModel, Set};
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate::state::{AppState, UserState};
use crate::telegram::handlers::HandleStatus;
use crate::user_service::UserService;

pub async fn handle_toggle_profanity_check(
    app: &AppState,
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
    app: &AppState,
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
