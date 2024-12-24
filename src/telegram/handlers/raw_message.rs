use teloxide::types::Message;

use super::HandleStatus;
use crate::state::{AppState, UserState};
use crate::telegram::actions;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let Some(text) = m.text() else {
        return Ok(HandleStatus::Skipped);
    };

    if text == "-" {
        return actions::dislike::handle(app, state, m).await;
    }

    Ok(HandleStatus::Skipped)
}
