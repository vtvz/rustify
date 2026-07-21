pub mod actions;
pub mod commands;
pub mod commands_admin;
pub mod handlers;
pub mod inline_buttons;
pub mod inline_buttons_actions;
pub mod inline_buttons_admin;
pub mod keyboards;
pub mod utils;

use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::ChatId;

use crate::app::App;
use crate::entity::prelude::UserModel;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::keyboards::StartKeyboard;

pub const MESSAGE_MAX_LEN: usize = 4096;

// TODO: Find a better place for this function
#[tracing::instrument(skip_all, fields(user_id = %user.id))]
pub async fn notify_token_invalid(app: &App, user: &UserModel) -> anyhow::Result<()> {
    app.bot()
        .send_message(
            ChatId(user.id.parse()?),
            t!(
                "error.spotify-invalid-token",
                locale = user.locale.as_ref(),
                command = UserCommandDisplay::Login,
            ),
        )
        .reply_markup(StartKeyboard::markup(user.locale.as_ref()))
        .await?;

    Ok(())
}
