use teloxide::prelude::*;
use teloxide::types::ParseMode;

use crate::app::App;
use crate::entity::prelude::*;
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

    app.bot()
        .send_message(m.chat.id, t!("language.changed", locale = locale))
        .reply_markup(StartKeyboard::markup(locale))
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(HandleStatus::Handled)
}
