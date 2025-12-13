use indoc::formatdoc;
use sea_orm::{Iterable, Order};
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::{TrackStatus, UserColumn, UserStatus};
use crate::services::{SpotifyPollingBackoffService, TrackStatusService, UserService};
use crate::telegram::commands_admin::AdminCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons_admin::{
    AdminInlineButtons,
    AdminUsersPageButtonType,
    AdminUsersSortBy,
    AdminUsersSortInfo,
    AdminUsersSortOrder,
};
use crate::user::UserState;
use crate::utils::DurationPrettyFormat as _;
use crate::utils::teloxide::CallbackQueryExt as _;

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

    let (text, keyboard) = build_users_page(
        app,
        state,
        0,
        AdminUsersSortBy::default(),
        AdminUsersSortOrder::default(),
        None,
    )
    .await?;

    app.bot()
        .send_message(m.chat.id, text)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(HandleStatus::Handled)
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), page, sort_by = ?sort_by, sort_order = ?sort_order, status_filter = ?status_filter))]
pub async fn handle_inline(
    app: &'static App,
    state: &UserState,
    q: CallbackQuery,
    page: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
    status_filter: Option<UserStatus>,
) -> anyhow::Result<()> {
    let Some(message) = q.get_message() else {
        app.bot()
            .answer_callback_query(q.id.clone())
            .text("Inaccessible Message")
            .await?;

        return Ok(());
    };

    let (text, keyboard) =
        build_users_page(app, state, page, sort_by, sort_order, status_filter).await?;

    app.bot()
        .edit_text(&message, text)
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
    status_filter: Option<UserStatus>,
) -> anyhow::Result<(String, InlineKeyboardMarkup)> {
    let total_users = UserService::count_users(app.db(), status_filter).await?;
    let total_pages = (total_users as f64 / USERS_PER_PAGE as f64).ceil() as u64;

    let users = UserService::get_users_paginated(
        app.db(),
        page,
        USERS_PER_PAGE,
        sort_by.into(),
        sort_order.into(),
        status_filter,
    )
    .await?;

    let mut message = vec![formatdoc!(
        r#"
        👥 <b>Recent Users</b>
        Page {page}/{total_pages} (Total: {total_users} users)

        Sorted by: <code>{sort_by:?} {sort_order:?}</code> | Filter: <code>{status}</code>
        "#,
        page = page + 1,
        status = status_filter
            .map(|status| format!("{status:?}"))
            .unwrap_or("All".into()),
    )];

    if users.is_empty() {
        message.push("<i>No users found</i>\n".into());
    }

    for (idx, user) in users.iter().enumerate() {
        let user_info = formatdoc!(
            r#"
                {index}. <b>{name}</b>
                • ID: <code>{id}</code> <a href="tg://user?id={id}">link</a>
                • Status: <code>{status:?}</code>
                • Lyrics Checked: <code>{lyrics_checked}</code>
                • Locale: <code>{locale}</code>
                • Created: <code>{created_at}</code>
            "#,
            index = page * USERS_PER_PAGE + idx as u64 + 1,
            name = user.name,
            id = user.id,
            status = user.status,
            lyrics_checked = user.lyrics_checked,
            locale = user.locale,
            created_at = user.created_at.format("%Y-%m-%d %H:%M:%S")
        );

        message.push(user_info);
    }

    message.push(format!(
        "You can get info about specific user by calling <code>/{command} [user-id]</code>",
        command = AdminCommandDisplay::Users
    ));

    let keyboard =
        create_pages_keyboard(state, page, total_pages, sort_by, sort_order, status_filter);

    Ok((message.join("\n"), keyboard))
}

fn create_pages_keyboard(
    state: &UserState,
    page: u64,
    total_pages: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
    status_filter: Option<UserStatus>,
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
            page: 0,
            button_type: AdminUsersPageButtonType::Sorting,
            sort_info: AdminUsersSortInfo {
                sort_by: AdminUsersSortBy::CreatedAt,
                sort_order: created_order,
                sort_selected: matches!(sort_by, AdminUsersSortBy::CreatedAt),
            },
            status_filter,
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
            page: 0,
            button_type: AdminUsersPageButtonType::Sorting,
            sort_info: AdminUsersSortInfo {
                sort_by: AdminUsersSortBy::LyricsChecked,
                sort_order: lyrics_order,
                sort_selected: matches!(sort_by, AdminUsersSortBy::LyricsChecked),
            },
            status_filter,
        }
        .into_inline_keyboard_button(state.locale()),
    );

    rows.push(sort_buttons);

    // Cycle through status filters: None (All) -> Active -> Pending -> ... -> None (All)
    // - If no filter (None): start with first status
    // - If filtered by a status: skip to current status, then take the next one
    // - If at the end: wrap around to None (showing all users)
    let next_filter = match status_filter {
        None => UserStatus::iter().next(),
        Some(current_status) => UserStatus::iter()
            .skip_while(|&s| s != current_status)
            .nth(1),
    };

    rows.push(vec![
        AdminInlineButtons::AdminUsersPage {
            page: 0,
            button_type: AdminUsersPageButtonType::Filter,
            sort_info: AdminUsersSortInfo {
                sort_by,
                sort_order,
                sort_selected: false,
            },
            status_filter: next_filter,
        }
        .into_inline_keyboard_button(state.locale()),
    ]);

    let mut navigation_buttons = vec![];

    if page > 0 {
        navigation_buttons.push(
            AdminInlineButtons::AdminUsersPage {
                page: page - 1,
                button_type: AdminUsersPageButtonType::Previous,
                sort_info: AdminUsersSortInfo {
                    sort_by,
                    sort_order,
                    sort_selected: false,
                },
                status_filter,
            }
            .into_inline_keyboard_button(state.locale()),
        );
    }

    if page + 1 < total_pages {
        navigation_buttons.push(
            AdminInlineButtons::AdminUsersPage {
                page: page + 1,
                button_type: AdminUsersPageButtonType::Next,
                sort_info: AdminUsersSortInfo {
                    sort_by,
                    sort_order,
                    sort_selected: false,
                },
                status_filter,
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
    let Some(user) = UserService::get_by_id(app.db(), user_id).await? else {
        let text = format!("User with ID <code>{user_id}</code> is not found");

        app.bot()
            .send_message(m.chat.id, text)
            .parse_mode(ParseMode::Html)
            .await?;

        return Ok(());
    };

    let stats = UserService::get_stats(app.db(), Some(user_id)).await?;

    let mut redis_conn = app.redis_conn().await?;
    let idle_duration =
        SpotifyPollingBackoffService::get_idle_duration(&mut redis_conn, user_id).await?;
    let last_activity =
        SpotifyPollingBackoffService::get_last_activity(&mut redis_conn, user_id).await?;
    let last_activity = chrono::DateTime::from_timestamp(last_activity, 0).unwrap_or_default();

    let suspend_time =
        SpotifyPollingBackoffService::get_suspend_time(&mut redis_conn, user_id).await?;

    let dislikes =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, Some(user_id), None)
            .await?;

    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, Some(user_id), None)
            .await?;

    let render_bool = |bool| if bool { "✅" } else { "❌" };

    let text = formatdoc!(
        r#"
            👤 <b>User Details</b>

            <b>Basic Info:</b>
            • Name: <b>{name}</b>
            • ID: <code>{id}</code> <a href="tg://user?id={id}">link</a>
            • Status: <code>{status:?}</code>
            • Role: <code>{role:?}</code>
            • Locale: <code>{locale}</code>
            • Created: <code>{created_at}</code>
            • Updated: <code>{updated_at}</code>

            <b>Configuration:</b>
            • Profanity Check: <code>{check_profanity}</code>
            • Track Skip: <code>{skip_tracks}</code>
            • Skippage Enabled: <code>{skippage_enabled}</code>
            • Skippage Duration: <code>{skippage_secs} seconds</code>
            • Magic Playlist: <code>{magic_playlist}</code>
            • Last Activity: <code>{last_activity}</code>
            • Idle Info: <code>{idle_duration}</code> / <code>{suspend_time}</code>

            <b>Statistics:</b>
            • Removed from Playlists: <code>{removed_playlists}</code>
            • Removed from Collection: <code>{removed_collection}</code>
            • Lyrics Checked: <code>{lyrics_checked}</code>
            • Lyrics Found: <code>{lyrics_found}</code>
            • Lyrics Profane: <code>{lyrics_profane}</code>
            • Lyrics Analyzed: <code>{lyrics_analyzed}</code>
            • Disliked Tracks: <code>{dislikes}</code>
            • Ignored Tracks: <code>{ignored}</code>

            <b>Lyrics Providers:</b>
            • Genius: <code>{lyrics_genius}</code>
            • MusixMatch: <code>{lyrics_musixmatch}</code>
            • LrcLib: <code>{lyrics_lrclib}</code>
        "#,
        name = user.name,
        id = user.id,
        status = user.status,
        role = user.role,
        locale = user.locale,
        created_at = user.created_at.format("%Y-%m-%d %H:%M:%S"),
        updated_at = user.updated_at.format("%Y-%m-%d %H:%M:%S"),
        check_profanity = render_bool(user.cfg_check_profanity),
        skip_tracks = render_bool(user.cfg_skip_tracks),
        skippage_enabled = render_bool(user.cfg_skippage_enabled),
        skippage_secs = user.cfg_skippage_secs,
        magic_playlist = user.magic_playlist.as_deref().unwrap_or("Not set"),
        last_activity = last_activity.format("%Y-%m-%d %H:%M:%S"),
        idle_duration = idle_duration.pretty_format(),
        suspend_time = suspend_time
            .map(|time| time.pretty_format())
            .unwrap_or("Inactive".into()),
        removed_playlists = stats.removed_playlists,
        removed_collection = stats.removed_collection,
        lyrics_checked = stats.lyrics_checked,
        lyrics_found = stats.lyrics_found,
        lyrics_profane = stats.lyrics_profane,
        lyrics_analyzed = stats.lyrics_analyzed,
        lyrics_genius = stats.lyrics_genius,
        lyrics_musixmatch = stats.lyrics_musixmatch,
        lyrics_lrclib = stats.lyrics_lrclib,
    );

    app.bot()
        .send_message(m.chat.id, text)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
