use anyhow::Result;
use teloxide::prelude::*;

use crate::state::UserState;

pub mod commands;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;

pub async fn handle_message(
    cx: UpdateWithCx<Bot, Message>,
    state: &UserState<'static>,
) -> Result<()> {
    let _ = commands::handle(&cx, state).await? || keyboards::handle(&cx, state).await?;

    Ok(())
}
