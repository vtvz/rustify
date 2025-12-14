pub mod details;
pub mod list;

use sea_orm::Order;
use teloxide::prelude::*;

use crate::app::App;
use crate::entity::prelude::UserColumn;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons_admin::{AdminUsersSortBy, AdminUsersSortOrder};
use crate::user::UserState;

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
        details::handle_command(app, state, m, &user_id).await?;

        return Ok(HandleStatus::Handled);
    }

    list::handle_command(app, state, m).await?;

    Ok(HandleStatus::Handled)
}
