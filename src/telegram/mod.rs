use commands::Command;
use handlers::HandleStatus;
use keyboards::StartKeyboard;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

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

use crate::app::App;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_message(
    app: &'static App,
    state: &UserState,
    m: Message,
) -> anyhow::Result<HandleStatus> {
    return_if_handled!(handlers::url::handle(app, state, &m).await?);

    // TODO: Better way to handle admin permissions
    if app.whitelist().is_admin(state.user_id()) {
        return_if_handled!(handlers::admin_commands::handle(app, state, &m).await?);
    }

    return_if_handled!(handlers::commands::handle(app, state, &m).await?);
    return_if_handled!(handlers::keyboards::handle(app, state, &m).await?);
    return_if_handled!(handlers::raw_message::handle(app, state, &m).await?);

    app.bot()
        .send_message(
            m.chat.id,
            Command::descriptions()
                .global_description(
                    "Your request was not handled 😔\n\nThere are commands available to you:",
                )
                .to_string(),
        )
        .reply_markup(StartKeyboard::markup())
        .await?;

    Ok(HandleStatus::Skipped)
}
