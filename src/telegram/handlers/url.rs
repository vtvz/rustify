use teloxide::types::Message;

use crate::state::{AppState, UserState};
use crate::telegram::actions;
use crate::telegram::utils::extract_url_from_message;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<bool> {
    if let Some(url) = extract_url_from_message(m) {
        let handled = actions::register::handle(app, state, &url, m).await?
            || actions::details::handle_url(app, state, &url, m).await?;

        if handled {
            return Ok(true);
        }
    };

    Ok(false)
}
