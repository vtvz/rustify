use teloxide::prelude::*;

use crate::app::App;
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %_state.user_id()))]
pub async fn handle(
    _app: &'static App,
    _state: &UserState,
    _m: &Message,
) -> anyhow::Result<HandleStatus> {
    Ok(HandleStatus::Handled)
}
