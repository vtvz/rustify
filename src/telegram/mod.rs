use handlers::HandleStatus;
use keyboards::StartKeyboard;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};

pub mod actions;
pub mod commands;
pub mod errors;
pub mod handlers;
pub mod inline_buttons;
pub mod keyboards;
pub mod utils;

pub const MESSAGE_MAX_LEN: usize = 4096;

macro_rules! return_if_handled {
    ($handle:expr) => {
        if matches!($handle, HandleStatus::Handled) {
            return Ok(HandleStatus::Handled);
        }
    };
}

pub(crate) use return_if_handled;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_message(
    app: &'static AppState,
    state: &UserState,
    m: Message,
) -> anyhow::Result<HandleStatus> {
    return_if_handled!(handlers::url::handle(app, state, &m).await?);
    return_if_handled!(handlers::commands::handle(app, state, &m).await?);
    return_if_handled!(handlers::keyboards::handle(app, state, &m).await?);
    return_if_handled!(handlers::raw_message::handle(app, state, &m).await?);

    app.bot()
        .send_message(m.chat.id, "You request was not handled 😔")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(HandleStatus::Skipped)
}
