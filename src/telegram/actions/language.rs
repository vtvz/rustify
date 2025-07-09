use teloxide::prelude::*;
use teloxide::types::{KeyboardRemove, ParseMode, ReplyMarkup};

use crate::app::App;
use crate::entity::prelude::*;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::keyboards::StartKeyboard;
use crate::user::UserState;
use crate::user_service::UserService;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
    locale: UserLocale,
) -> anyhow::Result<HandleStatus> {
    UserService::set_locale(app.db(), state.user_id(), locale.clone()).await?;

    let locale = locale.as_ref();

    if !state.is_spotify_authed().await {
        app.bot()
            .send_message(m.chat.id, t!("language.changed", locale = locale))
            .reply_markup(ReplyMarkup::KeyboardRemove(KeyboardRemove::new()))
            .parse_mode(ParseMode::Html)
            .await?;

        actions::register::send_register_invite(app, m.chat.id, locale).await?;
    } else {
        app.bot()
            .send_message(m.chat.id, t!("language.changed", locale = locale))
            .reply_markup(StartKeyboard::markup(locale))
            .parse_mode(ParseMode::Html)
            .await?;
    }

    Ok(HandleStatus::Handled)
}
