use teloxide::prelude::*;
use teloxide::types::{KeyboardRemove, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::UserService;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
    locale: UserLocale,
) -> anyhow::Result<HandleStatus> {
    UserService::set_locale(app.db(), state.user_id(), locale.clone()).await?;

    let state = app.user_state(state.user_id()).await?;

    let locale = locale.as_ref();

    if !state.is_spotify_authed().await {
        app.bot()
            .send_message(m.chat.id, t!("language.changed", locale = locale))
            .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
            .parse_mode(ParseMode::Html)
            .await?;

        actions::register::send_register_invite(app, &state).await?;
    } else {
        app.bot()
            .send_message(m.chat.id, t!("language.changed", locale = locale))
            .reply_markup(StartKeyboard::markup(locale))
            .parse_mode(ParseMode::Html)
            .await?;
    }

    Ok(HandleStatus::Handled)
}
