use rspotify::clients::OAuthClient;
use sea_orm::TransactionTrait;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId,
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use super::super::keyboards::StartKeyboard;
use crate::entity::prelude::*;
use crate::app::App;
use crate::services::UserService;
use crate::spotify::auth::SpotifyAuthService;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::handlers::HandleStatus;
use crate::user::UserState;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    url: &url::Url,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let value = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string());

    let Some(code) = value else {
        return Ok(HandleStatus::Skipped);
    };

    process_spotify_code(app, state, m, code).await
}

async fn process_spotify_code(
    app: &'static App,
    state: &UserState,
    m: &Message,
    code: String,
) -> anyhow::Result<HandleStatus> {
    let instance = state.spotify_write().await;

    if let Err(_err) = instance.request_token(&code).await {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "register.error",
                    command = UserCommandDisplay::Register,
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    }

    let token = instance.token.lock().await;

    let Ok(token) = token else {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "register.error",
                    command = UserCommandDisplay::Register,
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let Some(token) = token.clone() else {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "register.error",
                    command = UserCommandDisplay::Register,
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    {
        let txn = app.db().begin().await?;

        SpotifyAuthService::set_token(&txn, state.user_id(), token).await?;
        UserService::set_status(&txn, state.user_id(), UserStatus::Active).await?;

        txn.commit().await?;
    }

    app.bot()
        .send_message(
            m.chat.id,
            t!(
                "register.success",
                magic_command = UserCommandDisplay::Magic,
                skippage_command = UserCommandDisplay::Skippage,
                dislike_button = t!("start-keyboard.dislike", locale = state.locale()),
                details_button = t!("start-keyboard.details", locale = state.locale()),
                locale = state.locale()
            ),
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(StartKeyboard::markup(state.locale()))
        .await?;

    Ok(HandleStatus::Handled)
}

pub async fn send_register_invite(
    app: &'static App,
    chat_id: ChatId,
    locale: &str,
) -> anyhow::Result<HandleStatus> {
    let url = app.spotify_manager().get_authorize_url().await?;
    app.bot()
        .send_message(chat_id, t!("register.invite", locale = locale))
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineKeyboardButton {
                    text: t!("register.button", locale = locale).into(),
                    kind: InlineKeyboardButtonKind::Url(url.parse()?),
                }]
            ],
        )))
        .await?;

    Ok(HandleStatus::Handled)
}
