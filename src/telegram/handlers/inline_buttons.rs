use anyhow::Context as _;
use teloxide::prelude::*;

use crate::app::App;
use crate::telegram::actions;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::inline_buttons_admin::{AdminInlineButtons, AdminUsersSortInfo};
use crate::user::UserState;

#[tracing::instrument(
    skip_all,
    fields(
        user_id = %state.user_id(),
    )
)]
pub async fn handle(app: &'static App, state: &UserState, q: CallbackQuery) -> anyhow::Result<()> {
    let data = q.data.as_ref().context("Callback needs data")?;

    let admin_button: Result<AdminInlineButtons, _> = data.parse();

    if let Ok(button) = admin_button {
        if !state.user().is_admin() {
            app.bot()
                .answer_callback_query(q.id.clone())
                .text(t!("inline-buttons.alert-broken", locale = state.locale()))
                .show_alert(true)
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
            AdminInlineButtons::AdminUserSelect {
                user_id,
                page,
                sort_by,
                sort_order,
                status_filter,
                ..
            } => {
                actions::admin_users::details::handle_inline(
                    app,
                    state,
                    q,
                    user_id,
                    page,
                    sort_by,
                    sort_order,
                    status_filter,
                )
                .await?;
            },
            AdminInlineButtons::AdminUsersBack {
                page,
                sort_by,
                sort_order,
                status_filter,
            } => {
                actions::admin_users::list::handle_inline(
                    app,
                    state,
                    q,
                    page,
                    sort_by,
                    sort_order,
                    status_filter,
                )
                .await?;
            },
            AdminInlineButtons::AdminUsersPage {
                page,
                sort_info:
                    AdminUsersSortInfo {
                        sort_by,
                        sort_order,
                        ..
                    },
                status_filter,
                ..
            } => {
                actions::admin_users::list::handle_inline(
                    app,
                    state,
                    q,
                    page,
                    sort_by,
                    sort_order,
                    status_filter,
                )
                .await?;
            },
        }

        return Ok(());
    }

    if !state.is_spotify_authed().await {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text(t!("inline-buttons.alert-login", locale = state.locale()))
            .show_alert(true)
            .await?;

        actions::login::send_login_invite(app, state).await?;

        return Ok(());
    }

    let button: Result<InlineButtons, _> = data.parse();

    let button = match button {
        Ok(button) => button,
        Err(err) => {
            app.bot()
                .answer_callback_query(q.id.clone())
                .text(t!("inline-buttons.alert-broken", locale = state.locale()))
                .show_alert(true)
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
        InlineButtons::SongLinks(id) => {
            actions::song_links::handle_inline(app, state, q, &id).await?;
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
