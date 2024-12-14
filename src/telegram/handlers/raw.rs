use teloxide::types::Message;

use crate::state::{AppState, UserState};
use crate::telegram::actions;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<bool> {
    let handled = actions::register::handle(app, state, m).await?
        || actions::details::handle_url(app, state, m).await?;

    Ok(handled)
}
