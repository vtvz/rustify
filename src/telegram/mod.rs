use keyboards::StartKeyboard;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};

mod actions;
pub mod commands;
mod helpers;
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
    let handled = actions::register::handle(app, state, &m).await?
        || actions::details::handle_url(app, state, &m).await?
        || commands::handle(app, state, &m).await?
        || keyboards::handle(app, state, &m).await?;

    if !handled {
        app.bot()
            .send_message(m.chat.id, "You request was not handled 😔")
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;
    }

    Ok(())
}
