use teloxide::prelude::*;
use teloxide::types::{MediaKind, MessageCommon, MessageKind};

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
        // Handle simple text messages
        MessageKind::Common(MessageCommon {
            media_kind: MediaKind::Text(_),
            ..
        }) => {
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
            UserCommand::localized_descriptions(state.locale())
                .global_description(&t!("error.unhandled-request", locale = state.locale()))
                .to_string(),
        )
        .reply_markup(StartKeyboard::markup(state.locale()))
        .await?;

    Ok(HandleStatus::Skipped)
}
