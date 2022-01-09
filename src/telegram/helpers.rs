use anyhow::Result;
use rspotify::clients::OAuthClient;
use rspotify::ClientResult;
use teloxide::prelude::*;
use teloxide::types::{
    InlineKeyboardButton,
    InlineKeyboardButtonKind,
    InlineKeyboardMarkup,
    ParseMode,
    ReplyMarkup,
};

use crate::spotify;
use crate::spotify::CurrentlyPlaying;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::keyboards::StartKeyboard;
use crate::track_status_service;
use crate::track_status_service::TrackStatusService;

pub async fn handle_dislike(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<bool> {
    if !state.is_spotify_authed().await {
        return Ok(false);
    }

    let track = match spotify::currently_playing(&*state.spotify.read().await).await {
        CurrentlyPlaying::Err(err) => return Err(err),
        CurrentlyPlaying::None(message) => {
            cx.answer(message).send().await?;

            return Ok(true);
        }
        CurrentlyPlaying::Ok(track) => track,
    };

    let track_id = spotify::get_track_id(&track);

    TrackStatusService::set_status(
        &state.app.db,
        state.user_id.clone(),
        track_id.clone(),
        track_status_service::Status::Disliked,
    )
    .await?;

    cx.answer(format!("Disliked {}", spotify::create_track_name(&track)))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Cancel(track_id).into()]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}

pub async fn handle_register_invite(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState,
) -> Result<bool> {
    let url = state.app.spotify_manager.get_authorize_url().await?;
    cx.answer("Click this button below and after authentication copy URL from browser and send me")
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineKeyboardButton {
                    text: "Login with Spotify".into(),
                    kind: InlineKeyboardButtonKind::Url(url),
                }]
            ],
        )))
        .send()
        .await?;

    Ok(true)
}

pub async fn handle_register(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> Result<bool> {
    let Some(text) = cx.update.text() else {
        return Ok(false);
    };

    let Ok(url) = url::Url::parse(text) else {
        return Ok(false);
    };

    let code = loop {
        let Some((key, value)) = url.query_pairs().next() else {
            return Ok(false);
        };

        if key == "code" {
            break value.to_string();
        }
    };

    process_spotify_code(cx, state, code).await
}

pub async fn process_spotify_code(
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

    SpotifyAuthService::set_token(&state.app.db, state.user_id.clone(), token).await?;

    cx.answer("Yeah! You registered successfully!")
        .reply_markup(StartKeyboard::markup())
        .send()
        .await?;

    Ok(true)
}
