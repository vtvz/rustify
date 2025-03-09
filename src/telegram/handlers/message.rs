use teloxide::prelude::*;
use teloxide::types::MessageKind;
use teloxide::utils::command::BotCommands;

use super::{HandleStatus, return_if_handled};
use crate::app::App;
use crate::telegram::commands::UserCommand;
use crate::telegram::handlers;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: Message,
) -> anyhow::Result<HandleStatus> {
    #[allow(clippy::single_match)]
    match m.kind {
        MessageKind::Common(_) => {
            return_if_handled!(handlers::url::handle(app, state, &m).await?);

            // TODO: Better way to handle admin permissions
            if app.whitelist().is_admin(state.user_id()) {
                return_if_handled!(handlers::admin_commands::handle(app, state, &m).await?);
            }

            return_if_handled!(handlers::commands::handle(app, state, &m).await?);
            return_if_handled!(handlers::keyboards::handle(app, state, &m).await?);
            return_if_handled!(handlers::raw_message::handle(app, state, &m).await?);
        },
        _ => {},
    }

    app.bot()
        .send_message(
            m.chat.id,
            UserCommand::descriptions()
                .global_description(
                    "Your request was not handled 😔\n\nThere are commands available to you:",
                )
                .to_string(),
        )
        .reply_markup(StartKeyboard::markup())
        .await?;

    Ok(HandleStatus::Skipped)
}
