use keyboards::StartKeyboard;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};

pub mod actions;
pub mod commands;
pub mod handlers;
pub mod inline_buttons;
pub mod keyboards;
pub mod utils;

pub const MESSAGE_MAX_LEN: usize = 4096;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_message(
    app: &'static AppState,
    state: &UserState,
    m: Message,
) -> anyhow::Result<()> {
    let handled = handlers::url::handle(app, state, &m).await?
        || handlers::commands::handle(app, state, &m).await?
        || handlers::keyboards::handle(app, state, &m).await?;

    if !handled {
        app.bot()
            .send_message(m.chat.id, "You request was not handled 😔")
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;
    }

    Ok(())
}
