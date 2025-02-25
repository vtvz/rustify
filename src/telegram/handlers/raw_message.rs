use teloxide::types::Message;

use super::HandleStatus;
use crate::app::App;
use crate::telegram::actions;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let Some(text) = m.text() else {
        return Ok(HandleStatus::Skipped);
    };

    if text == "-" {
        return actions::dislike::handle(app, state, m).await;
    }

    if text == "+" {
        return actions::like::handle(app, state, m).await;
    }

    Ok(HandleStatus::Skipped)
}
