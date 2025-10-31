use anyhow::Context as _;
use indoc::formatdoc;
use sea_orm::Order;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::UserColumn;
use crate::services::UserService;
use crate::telegram::commands_admin::AdminCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons_admin::{
    AdminInlineButtons,
    AdminUsersPageButtonType,
    AdminUsersSortBy,
    AdminUsersSortOrder,
};
use crate::user::UserState;

const USERS_PER_PAGE: u64 = 10;

impl From<AdminUsersSortBy> for UserColumn {
    fn from(sort: AdminUsersSortBy) -> Self {
        match sort {
            AdminUsersSortBy::CreatedAt => UserColumn::CreatedAt,
            AdminUsersSortBy::LyricsChecked => UserColumn::LyricsChecked,
        }
    }
}

impl From<AdminUsersSortOrder> for Order {
    fn from(order: AdminUsersSortOrder) -> Self {
        match order {
            AdminUsersSortOrder::Asc => Order::Asc,
            AdminUsersSortOrder::Desc => Order::Desc,
        }
    }
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_command(
    app: &'static App,
    state: &UserState,
    m: &Message,
    user_id: String,
) -> anyhow::Result<HandleStatus> {
    if !user_id.is_empty() {
        show_user_details(app, m, &user_id).await?;

        return Ok(HandleStatus::Handled);
    }

    let (message, keyboard) = build_users_page(
        app,
        state,
        0,
        AdminUsersSortBy::default(),
        AdminUsersSortOrder::default(),
    )
    .await?;

    app.bot()
        .send_message(m.chat.id, message)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), page, sort_by = ?sort_by, sort_order = ?sort_order))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    page: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
) -> anyhow::Result<()> {
    app.bot().answer_callback_query(q.id).await?;

    let m = q.message.context("Should have message")?;

    let chat_id = m.chat().id;
    let message_id = m.id();

    let (text, keyboard) = build_users_page(app, state, page, sort_by, sort_order).await?;

    app.bot()
        .edit_message_text(chat_id, message_id, text)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn build_users_page(
    app: &'static App,
    state: &UserState,
    page: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let total_users = UserService::count_users(app.db(), None).await?;
    let total_pages = (total_users as f64 / USERS_PER_PAGE as f64).ceil() as u64;

    let users = UserService::get_users_paginated(
        app.db(),
        page,
        USERS_PER_PAGE,
        sort_by.into(),
        sort_order.into(),
    )
    .await?;

    let mut message = vec![formatdoc!(
        r#"
        👥 <b>Recent Users</b>
        Page {}/{} (Total: {} users)

        Sorted by: <code>{:?} {:?}</code>
        "#,
        page + 1,
        total_pages,
        total_users,
        sort_by,
        sort_order,
    )];

    for (idx, user) in users.iter().enumerate() {
        let user_info = formatdoc!(
            r#"
            {}. <b>{}</b>
            • ID: <code>{}</code>
            • Status: <code>{:?}</code>
            • Lyrics Checked: <code>{}</code>
            • Locale: <code>{}</code>
            • Created: <code>{}</code>
            "#,
            page * USERS_PER_PAGE + idx as u64 + 1,
            user.name,
            user.id,
            user.status,
            user.lyrics_checked,
            user.locale,
            user.created_at.format("%Y-%m-%d %H:%M:%S")
        );

        message.push(user_info);
    }

    message.push(format!(
        "You can get info about specific user by calling <code>/{command} [user-id]</code>",
        command = AdminCommandDisplay::Users
    ));

    let keyboard = create_pages_keyboard(state, page, sort_by, sort_order, total_pages);

    Ok((message.join("\n"), keyboard))
}

fn create_pages_keyboard(
    state: &UserState,
    page: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
    total_pages: u64,
) -> InlineKeyboardMarkup {
    let mut rows = vec![];

    let mut sort_buttons = vec![];

    let created_order = if sort_by == AdminUsersSortBy::CreatedAt {
        !sort_order
    } else {
        AdminUsersSortOrder::default()
    };

    sort_buttons.push(
        AdminInlineButtons::AdminUsersPage {
            page,
            button_type: AdminUsersPageButtonType::Sorting,
            sort_by: AdminUsersSortBy::CreatedAt,
            sort_order: created_order,
            selected_order: matches!(sort_by, AdminUsersSortBy::CreatedAt),
        }
        .into_inline_keyboard_button(state.locale()),
    );

    let lyrics_order = if sort_by == AdminUsersSortBy::LyricsChecked {
        !sort_order
    } else {
        AdminUsersSortOrder::default()
    };

    sort_buttons.push(
        AdminInlineButtons::AdminUsersPage {
            page,
            button_type: AdminUsersPageButtonType::Sorting,
            sort_by: AdminUsersSortBy::LyricsChecked,
            sort_order: lyrics_order,
            selected_order: matches!(sort_by, AdminUsersSortBy::LyricsChecked),
        }
        .into_inline_keyboard_button(state.locale()),
    );

    rows.push(sort_buttons);

    let mut navigation_buttons = vec![];

    if page > 0 {
        navigation_buttons.push(
            AdminInlineButtons::AdminUsersPage {
                page: page - 1,
                button_type: AdminUsersPageButtonType::Previous,
                sort_by,
                sort_order,
                selected_order: false,
            }
            .into_inline_keyboard_button(state.locale()),
        );
    }

    if page + 1 < total_pages {
        navigation_buttons.push(
            AdminInlineButtons::AdminUsersPage {
                page: page + 1,
                button_type: AdminUsersPageButtonType::Next,
                sort_by,
                sort_order,
                selected_order: false,
            }
            .into_inline_keyboard_button(state.locale()),
        );
    }

    if !navigation_buttons.is_empty() {
        rows.push(navigation_buttons);
    }

    InlineKeyboardMarkup::new(rows)
}

async fn show_user_details(app: &'static App, m: &Message, user_id: &str) -> anyhow::Result<()> {
    let user = UserService::obtain_by_id(app.db(), user_id).await?;
    let stats = UserService::get_stats(app.db(), Some(user_id)).await?;

    let render_bool = |bool| if bool { "✅" } else { "❌" };

    let message = formatdoc!(
        r#"
        👤 <b>User Details</b>

        <b>Basic Info:</b>
        • Name: <b>{}</b>
        • ID: <code>{}</code>
        • Status: <code>{:?}</code>
        • Role: <code>{:?}</code>
        • Locale: <code>{}</code>
        • Created: <code>{}</code>
        • Updated: <code>{}</code>

        <b>Configuration:</b>
        • Profanity Check: <code>{}</code>
        • Track Skip: <code>{}</code>
        • Skippage Enabled: <code>{}</code>
        • Skippage Duration: <code>{} seconds</code>
        • Magic Playlist: <code>{}</code>

        <b>Statistics:</b>
        • Removed from Playlists: <code>{}</code>
        • Removed from Collection: <code>{}</code>
        • Lyrics Checked: <code>{}</code>
        • Lyrics Found: <code>{}</code>
        • Lyrics Profane: <code>{}</code>
        • Lyrics Analyzed: <code>{}</code>

        <b>Lyrics Providers:</b>
        • Genius: <code>{}</code>
        • MusixMatch: <code>{}</code>
        • LrcLib: <code>{}</code>
        "#,
        user.name,
        user.id,
        user.status,
        user.role,
        user.locale,
        user.created_at.format("%Y-%m-%d %H:%M:%S"),
        user.updated_at.format("%Y-%m-%d %H:%M:%S"),
        render_bool(user.cfg_check_profanity),
        render_bool(user.cfg_skip_tracks),
        render_bool(user.cfg_skippage_enabled),
        user.cfg_skippage_secs,
        user.magic_playlist.as_deref().unwrap_or("Not set"),
        stats.removed_playlists,
        stats.removed_collection,
        stats.lyrics_checked,
        stats.lyrics_found,
        stats.lyrics_profane,
        stats.lyrics_analyzed,
        stats.lyrics_genius,
        stats.lyrics_musixmatch,
        stats.lyrics_lrclib,
    );

    app.bot()
        .send_message(m.chat.id, message)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
