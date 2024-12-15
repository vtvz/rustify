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
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state::{AppState, UserState};
use crate::user_service::UserService;

pub async fn handle(
    app: &'static AppState,
    state: &UserState,
    url: &reqwest::Url,
    m: &Message,
) -> anyhow::Result<bool> {
    let value = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string());

    let Some(code) = value else {
        return Ok(false);
    };

    process_spotify_code(app, state, m, code).await
}

async fn process_spotify_code(
    app: &'static AppState,
    state: &UserState,
    m: &Message,
    code: String,
) -> anyhow::Result<bool> {
    let instance = state.spotify_write().await;

    if let Err(err) = instance.request_token(&code).await {
        app.bot().send_message(m.chat.id, "Cannot retrieve token. Code is probably broken. Run /register command and try again please")
            .send()
            .await?;

        return Err(err.into());
    }

    let token = instance.token.lock().await;

    let Ok(token) = token else {
        app.bot()
            .send_message(m.chat.id, "Cannot retrieve token. Try again")
            .send()
            .await?;

        return Ok(true);
    };

    let Some(token) = token.clone() else {
        app.bot()
            .send_message(m.chat.id, "Token is not retrieved. Try again")
            .send()
            .await?;

        return Ok(true);
    };

    {
        let txn = app.db().begin().await?;

        SpotifyAuthService::set_token(&txn, state.user_id(), token).await?;
        UserService::set_status(&txn, state.user_id(), UserStatus::Active).await?;

        txn.commit().await?;
    }

    app.bot()
        .send_message(m.chat.id, "Yeah! You registered successfully!")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(true)
}

pub async fn send_register_invite(app: &'static AppState, chat_id: ChatId) -> anyhow::Result<bool> {
    let url = app.spotify_manager().get_authorize_url().await?;
    app.bot()
        .send_message(
            chat_id,
            "Click this button below and after authentication copy URL from browser and send me",
        )
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineKeyboardButton {
                    text: "Login with Spotify".into(),
                    kind: InlineKeyboardButtonKind::Url(url.parse()?),
                }]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}
