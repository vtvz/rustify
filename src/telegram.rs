use anyhow::Result;
use teloxide::prelude::*;

use crate::state::UserState;

pub mod commands;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;

pub async fn handle_message(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<()> {
    let handled = helpers::handle_register(cx, state).await?
        || commands::handle(cx, state).await?
        || keyboards::handle(cx, state).await?;

    if !handled {
        cx.answer("You request is not handled 😔").send().await?;
    }

    Ok(())
}
