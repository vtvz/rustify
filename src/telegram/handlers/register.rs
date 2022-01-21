use anyhow::Result;
use rspotify::clients::OAuthClient;
use teloxide::prelude::*;

use crate::state::UserState;
use crate::telegram::keyboards::StartKeyboard;
use crate::SpotifyAuthService;

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<bool> {
    let Some(text) = cx.update.text() else {
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

    process_spotify_code(cx, state, code).await
}

async fn process_spotify_code(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState,
    code: String,
) -> Result<bool> {
    let mut instance = state.spotify.write().await;

    if let Err(err) = instance.request_token(&code).await {
        cx.answer("Cannot retrieve token. Code is probably broken. Run /register command and try again please")
            .send()
            .await?;

        return Err(err.into());
    }

    let token = instance.token.lock().await;

    let Ok(token) = token else {
        cx.answer("Cannot retrieve token. Try again").send().await?;

        return Ok(true);
    };

    let Some(token) = token.clone() else {
        cx.answer("Token is not retrieved. Try again").send().await?;

        return Ok(true);
    };

    SpotifyAuthService::set_token(&state.app.db, &state.user_id, token).await?;

    cx.answer("Yeah! You registered successfully!")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(true)
}
