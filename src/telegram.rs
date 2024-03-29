use keyboards::StartKeyboard;
use teloxide::prelude::*;

use crate::state::UserState;

pub mod commands;
mod handlers;
mod helpers;
pub mod inline_buttons;
pub mod keyboards;

pub const MESSAGE_MAX_LEN: usize = 4096;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id))]
pub async fn handle_message(m: Message, bot: Bot, state: &UserState) -> anyhow::Result<()> {
    let handled = handlers::register::handle(&m, &bot, state).await?
        || handlers::details::handle_url(&m, &bot, state).await?
        || commands::handle(&m, &bot, state).await?
        || keyboards::handle(&m, &bot, state).await?;

    if !handled {
        bot.send_message(m.chat.id, "You request was not handled 😔")
            .reply_markup(StartKeyboard::markup())
            .send()
            .await?;
    }

    Ok(())
}
