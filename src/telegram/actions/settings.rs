use sea_orm::{ActiveModelTrait as _, IntoActiveModel as _, Set};
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = state.user_id()))]
pub async fn handle_toggle_profanity_check(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let new_status = !state.user().cfg_check_profanity;

    let mut user_model = state.user().clone().into_active_model();

    user_model.cfg_check_profanity = Set(new_status);
    user_model.save(app.db()).await?;

    let text = if new_status {
        t!("settings.profanity-check-on", locale = state.locale())
    } else {
        t!("settings.profanity-check-off", locale = state.locale())
    };

    app.bot()
        .send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(user_id = state.user_id()))]
pub async fn handle_toggle_skip_tracks(
    app: &App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let new_status = !state.user().cfg_skip_tracks;

    let mut user_model = state.user().clone().into_active_model();

    user_model.cfg_skip_tracks = Set(new_status);
    user_model.save(app.db()).await?;

    let text = if new_status {
        t!("settings.skip-on", locale = state.locale())
    } else {
        t!("settings.skip-off", locale = state.locale())
    };

    app.bot()
        .send_message(chat_id, text)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
