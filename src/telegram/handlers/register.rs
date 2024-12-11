use rspotify::clients::OAuthClient;
use sea_orm::TransactionTrait;
use teloxide::prelude::*;

use super::super::keyboards::StartKeyboard;
use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state::{AppState, UserState};
use crate::telegram::utils::extract_url_from_message;
use crate::user_service::UserService;

pub async fn handle(
    app_state: &'static AppState,
    state: &UserState,
    bot: &Bot,
    m: &Message,
) -> anyhow::Result<bool> {
    let Some(url) = extract_url_from_message(m) else {
        return Ok(false);
    };

    let value = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string());

    let Some(code) = value else {
        return Ok(false);
    };

    process_spotify_code(app_state, state, bot, m, code).await
}

async fn process_spotify_code(
    app_state: &'static AppState,
    state: &UserState,
    bot: &Bot,
    m: &Message,
    code: String,
) -> anyhow::Result<bool> {
    let instance = state.spotify_write().await;

    if let Err(err) = instance.request_token(&code).await {
        bot.send_message(m.chat.id, "Cannot retrieve token. Code is probably broken. Run /register command and try again please")
            .send()
            .await?;

        return Err(err.into());
    }

    let token = instance.token.lock().await;

    let Ok(token) = token else {
        bot.send_message(m.chat.id, "Cannot retrieve token. Try again")
            .send()
            .await?;

        return Ok(true);
    };

    let Some(token) = token.clone() else {
        bot.send_message(m.chat.id, "Token is not retrieved. Try again")
            .send()
            .await?;

        return Ok(true);
    };

    {
        let txn = app_state.db().begin().await?;

        SpotifyAuthService::set_token(&txn, state.user_id(), token).await?;
        UserService::set_status(&txn, state.user_id(), UserStatus::Active).await?;

        txn.commit().await?;
    }

    bot.send_message(m.chat.id, "Yeah! You registered successfully!")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(true)
}
