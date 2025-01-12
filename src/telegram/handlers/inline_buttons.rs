use anyhow::Context;
use teloxide::dispatching::dialogue::GetChatId as _;
use teloxide::prelude::*;

use crate::state::{AppState, UserState};
use crate::telegram::actions;
use crate::telegram::inline_buttons::InlineButtons;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    q: CallbackQuery,
) -> anyhow::Result<()> {
    if !state.is_spotify_authed().await {
        app.bot()
            .answer_callback_query(&q.id)
            .text("You need to register first")
            .await?;

        if let Some(chat_id) = q.chat_id() {
            actions::register::send_register_invite(app, chat_id).await?;
        }

        return Ok(());
    }

    let data = q.data.as_ref().context("Callback needs data")?;

    let button: InlineButtons = data.parse()?;

    match button {
        InlineButtons::Cancel(id) => {
            actions::cancel::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Dislike(id) => {
            actions::dislike::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Ignore(id) => {
            actions::ignore::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Analyze(_) => {
            app.bot()
                .answer_callback_query(q.id)
                .text("Work In Progress")
                .await?;
        },
    }

    Ok(())
}
