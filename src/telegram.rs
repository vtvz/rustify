use anyhow::Result;
use teloxide::prelude::*;

use crate::state::UserState;
use crate::telegram::keyboards::StartKeyboard;

pub mod commands;
mod handlers;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;

pub const MESSAGE_MAX_LEN: usize = 4096;

pub async fn handle_message(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<()> {
    let handled = handlers::register::handle(cx, state).await?
        || handlers::details::handle_url(cx, state).await?
        || commands::handle(cx, state).await?
        || keyboards::handle(cx, state).await?;

    if !handled {
        cx.answer("You request is not handled 😔")
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;
    }

    Ok(())
}
