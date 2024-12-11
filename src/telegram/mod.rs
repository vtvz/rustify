use keyboards::StartKeyboard;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};

pub mod commands;
mod handlers;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;
pub mod utils;

pub const MESSAGE_MAX_LEN: usize = 4096;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_message(
    app_state: &'static AppState,
    state: &UserState,
    bot: Bot,
    m: Message,
) -> anyhow::Result<()> {
    let handled = handlers::register::handle(app_state, state, &bot, &m).await?
        || handlers::details::handle_url(app_state, state, &bot, &m).await?
        || commands::handle(app_state, state, &bot, &m).await?
        || keyboards::handle(app_state, state, &bot, &m).await?;

    if !handled {
        bot.send_message(m.chat.id, "You request was not handled 😔")
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;
    }

    Ok(())
}
