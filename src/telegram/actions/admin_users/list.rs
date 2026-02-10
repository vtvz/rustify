use indoc::formatdoc;
use sea_orm::Iterable as _;
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::{UserModel, UserStatus};
use crate::services::UserService;
use crate::telegram::commands_admin::AdminCommandDisplay;
use crate::telegram::inline_buttons_admin::{
    AdminInlineButtons,
    AdminUsersPageButtonType,
    AdminUsersSortBy,
    AdminUsersSortInfo,
    AdminUsersSortOrder,
};
use crate::user::UserState;
use crate::utils::teloxide::CallbackQueryExt as _;

const USERS_PER_PAGE: u64 = 10;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_command(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<()> {
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

    Ok(())
}

#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), %page, ?sort_by, ?sort_order, ?status_filter))]
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

pub async fn build_users_page(
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
        r"
        👥 <b>Recent Users</b>
        Page {page}/{total_pages} (Total: {total_users} users)

        Sorted by: <code>{sort_by:?} {sort_order:?}</code> | Filter: <code>{status}</code>
        ",
        page = page + 1,
        status = status_filter.map_or("All".into(), |status| format!("{status:?}")),
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

    let keyboard = create_pages_keyboard(
        state,
        page,
        total_pages,
        sort_by,
        sort_order,
        status_filter,
        &users,
    );

    Ok((message.join("\n"), keyboard))
}

fn create_pages_keyboard(
    state: &UserState,
    page: u64,
    total_pages: u64,
    sort_by: AdminUsersSortBy,
    sort_order: AdminUsersSortOrder,
    status_filter: Option<UserStatus>,
    users: &[UserModel],
) -> InlineKeyboardMarkup {
    let mut rows = vec![];

    if !users.is_empty() {
        let numbered_buttons: Vec<_> = users
            .iter()
            .enumerate()
            .map(|(idx, user)| {
                AdminInlineButtons::AdminUserSelect {
                    idx: idx as u8,
                    user_id: user.id.clone(),
                    page,
                    sort_by,
                    sort_order,
                    status_filter,
                }
                .into_inline_keyboard_button(state.locale())
            })
            .collect();

        for chunk in numbered_buttons.chunks(5) {
            rows.push(chunk.to_vec());
        }
    }

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
