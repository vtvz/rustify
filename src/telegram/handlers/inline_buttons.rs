use anyhow::Context as _;
use teloxide::dispatching::dialogue::GetChatId as _;
use teloxide::prelude::*;

use crate::app::App;
use crate::telegram::actions;
use crate::telegram::inline_buttons::{AdminInlineButtons, InlineButtons};
use crate::user::UserState;

#[tracing::instrument(
    skip_all,
    fields(
        user_id = state.user_id(),
    )
)]
pub async fn handle(app: &'static App, state: &UserState, q: CallbackQuery) -> anyhow::Result<()> {
    if !state.is_spotify_authed().await {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("You need to register first")
            .await?;

        if let Some(chat_id) = q.chat_id() {
            actions::register::send_register_invite(app, chat_id, state.locale()).await?;
        }

        return Ok(());
    }

    let data = q.data.as_ref().context("Callback needs data")?;

    let admin_button: Result<AdminInlineButtons, _> = data.parse();

    if let Ok(button) = admin_button {
        if !state.user().is_admin() {
            app.bot()
                .answer_callback_query(q.id.clone())
                .text("Button is broken. Try another one")
                .await?;

            return Ok(());
        }

        match button {
            AdminInlineButtons::RegenerateWordDefinition { locale, word } => {
                actions::word_definition::handle_inline_regenerate(app, q, locale, word).await?;
            },
            AdminInlineButtons::WordDefinitionsPage { locale, page, .. } => {
                actions::word_definition::handle_inline_list(app, q, locale, page).await?;
            },
        }

        return Ok(());
    }

    let button: Result<InlineButtons, _> = data.parse();

    let button = match button {
        Ok(button) => button,
        Err(err) => {
            app.bot()
                .answer_callback_query(q.id.clone())
                .text("Button is broken. Try another one")
                .await?;

            tracing::error!(err = ?err, data, "Error parsing user inline button");

            return Ok(());
        },
    };

    match button {
        InlineButtons::Dislike(id) => {
            actions::dislike::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Ignore(id) => {
            actions::ignore::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Analyze(id) => {
            actions::analyze::handle_inline(app, state, q, &id).await?;
        },
        InlineButtons::Magic => {
            actions::magic::handle_inline(app, state, q).await?;
        },
        InlineButtons::Recommendasion => {
            actions::recommendasion::handle_inline(app, state, q).await?;
        },
        InlineButtons::SkippageEnable(to_enable) => {
            actions::skippage::handle_inline(app, state, q, to_enable).await?;
        },
    }

    Ok(())
}
