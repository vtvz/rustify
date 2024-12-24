use teloxide::types::Message;

use super::HandleStatus;
use crate::state::{AppState, UserState};
use crate::telegram::utils::extract_url_from_message;
use crate::telegram::{actions, return_if_handled};

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    if let Some(url) = extract_url_from_message(m) {
        return_if_handled!(actions::register::handle(app, state, &url, m).await?);
        return_if_handled!(actions::details::handle_url(app, state, &url, m).await?);
    };

    Ok(HandleStatus::Skipped)
}
