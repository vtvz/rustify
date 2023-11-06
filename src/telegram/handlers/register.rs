use rspotify::clients::OAuthClient;
use sea_orm::TransactionTrait;
use teloxide::prelude::*;

use super::super::keyboards::StartKeyboard;
use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state::UserState;
use crate::user_service::UserService;

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let Some(text) = m.text() else {
        return Ok(false);
    };

    let Ok(url) = url::Url::parse(text) else {
        return Ok(false);
    };

    let value = url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.to_string());

    let Some(code) = value else {
        return Ok(false);
    };

    process_spotify_code(m, bot, state, code).await
}

async fn process_spotify_code(
    m: &Message,
    bot: &Bot,
    state: &UserState,
    code: String,
) -> anyhow::Result<bool> {
    let instance = state.spotify.write().await;

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
        let txn = state.app.db().begin().await?;

        SpotifyAuthService::set_token(&txn, &state.user_id, token).await?;
        UserService::set_status(&txn, &state.user_id, UserStatus::Active).await?;

        txn.commit().await?;
    }

    bot.send_message(m.chat.id, "Yeah! You registered successfully!")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(true)
}
