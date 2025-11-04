use teloxide::prelude::*;
use teloxide::types::{
    ChatId,
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use crate::app::App;
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

#[tracing::instrument(skip_all)]
pub async fn send_login_invite(
    app: &'static App,
    state: &UserState,
) -> anyhow::Result<HandleStatus> {
    let url = app.spotify_manager().get_authorize_url(state).await?;
    app.bot()
        .send_message(
            ChatId(state.user_id().parse()?),
            t!("login.invite", locale = state.locale()),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineKeyboardButton {
                    text: t!("login.button", locale = state.locale()).into(),
                    kind: InlineKeyboardButtonKind::Url(url.parse()?),
                }]
            ],
        )))
        .await?;

    Ok(HandleStatus::Handled)
}
