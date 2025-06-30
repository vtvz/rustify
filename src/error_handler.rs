use anyhow::Context;
use reqwest::StatusCode;
use teloxide::ApiError;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::types::ChatId;

use crate::app::App;
use crate::entity::prelude::UserStatus;
use crate::spotify;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::keyboards::StartKeyboard;
use crate::user_service::UserService;

#[derive(Default)]
pub struct ErrorHandlingResult {
    pub handled: bool,
    pub user_notified: bool,
}

impl ErrorHandlingResult {
    pub fn unhandled() -> Self {
        ErrorHandlingResult {
            handled: false,
            user_notified: false,
        }
    }

    pub fn handled() -> Self {
        ErrorHandlingResult {
            handled: true,
            user_notified: false,
        }
    }

    pub fn handled_notified() -> Self {
        ErrorHandlingResult {
            handled: true,
            user_notified: true,
        }
    }
}

#[tracing::instrument(skip_all, fields(user_id = user_id))]
pub async fn handle_blocked_bot(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let Some(err) = err.downcast_mut::<teloxide::RequestError>() else {
        return Ok(ErrorHandlingResult::unhandled());
    };

    if let teloxide::RequestError::Api(ApiError::BotBlocked) = err {
        UserService::set_status(app.db(), user_id, UserStatus::Blocked).await?;

        return Ok(ErrorHandlingResult::handled_notified());
    }

    return Ok(ErrorHandlingResult::unhandled());
}

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn spotify_resp_error(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
    locale: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let Some(response) = spotify::SpotifyError::extract_response(err) else {
        return Ok(ErrorHandlingResult::unhandled());
    };

    if response.status() == StatusCode::TOO_MANY_REQUESTS {
        tracing::info!("User got a 429 error (too many requests)");

        let header = response
            .headers()
            .get("Retry-After")
            .context("Need Retry-After header to proceed")?;

        let retry_after: i64 = header.to_str()?.parse()?;

        SpotifyAuthService::suspend_for(
            app.db(),
            &[user_id],
            chrono::Duration::seconds(retry_after),
        )
        .await?;
    }

    match spotify::SpotifyError::from_anyhow(err).await {
        Ok(Some(err)) => {
            match err {
                spotify::SpotifyError::Regular(serr) => {
                    if serr.error.status == 500 {
                        // NOTE: Ignore these errors
                        // They are just spam

                        return Ok(ErrorHandlingResult::handled());
                    }

                    if serr.error.status == 403
                        && serr.error.message == "Spotify is unavailable in this country"
                    {
                        tracing::error!(err = ?serr, "Spotify is unavailable in this country. Stopping user checks");

                        UserService::set_status(app.db(), user_id, UserStatus::Forbidden).await?;

                        app.bot()
                            .send_message(
                                ChatId(user_id.parse()?),
                                t!(
                                    "error.unavailable-in-country",
                                    command = UserCommandDisplay::Register
                                ),
                            )
                            .reply_markup(StartKeyboard::markup(locale))
                            .await?;

                        return Ok(ErrorHandlingResult::handled_notified());
                    }

                    // TODO: I need to inspect what errors to notify
                    tracing::error!(err = ?serr, "Regular Spotify Error Happened");

                    return Ok(ErrorHandlingResult::handled());
                },
                spotify::SpotifyError::Auth(serr) => {
                    tracing::error!(err = ?serr, "Auth Spotify Error");

                    UserService::set_status(app.db(), user_id, UserStatus::TokenInvalid).await?;

                    app.bot()
                        .send_message(
                            ChatId(user_id.parse()?),
                            t!(
                                "error.spotify-auth-failed",
                                command = UserCommandDisplay::Register,
                                error = serr.error_description
                            ),
                        )
                        .reply_markup(StartKeyboard::markup(locale))
                        .await?;

                    return Ok(ErrorHandlingResult::handled_notified());
                },
            };
        },
        Err(err) => {
            tracing::error!(err = ?err, "Had an issue parsing responce");

            return Ok(ErrorHandlingResult::handled());
        },
        Ok(None) => return Ok(ErrorHandlingResult::unhandled()),
    };
}

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn spotify_client_error(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
    locale: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let Some(err) = err.downcast_mut::<rspotify::ClientError>() else {
        return Ok(ErrorHandlingResult::unhandled());
    };

    match err {
        rspotify::ClientError::InvalidToken => {
            tracing::error!(err = ?err, "User has Invalid Spotify Token");
            UserService::set_status(app.db(), user_id, UserStatus::TokenInvalid).await?;

            app.bot()
                .send_message(
                    ChatId(user_id.parse()?),
                    t!(
                        "error.spotify-invalid-token",
                        command = UserCommandDisplay::Register
                    ),
                )
                .reply_markup(StartKeyboard::markup(locale))
                .await?;

            return Ok(ErrorHandlingResult::handled_notified());
        },
        _ => tracing::error!(err = ?err, "Spotify Client Has an Issue"),
    }

    Ok(ErrorHandlingResult::handled())
}

#[tracing::instrument(skip_all, fields(user_id))]
async fn handle_inner(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
    locale: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let res = spotify_resp_error(err, app, user_id, locale).await?;
    if res.handled {
        return Ok(res);
    }

    let res = spotify_client_error(err, app, user_id, locale).await?;
    if res.handled {
        return Ok(res);
    }

    let res = handle_blocked_bot(err, app, user_id).await?;
    if res.handled {
        return Ok(res);
    }

    tracing::error!(err = ?err, "Unhandled Error");

    Ok(ErrorHandlingResult::unhandled())
}

#[tracing::instrument(skip_all, fields(user_id))]
pub async fn handle(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
    locale: &str,
) -> ErrorHandlingResult {
    let res = handle_inner(err, app, user_id, locale).await;

    match res {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(err = ?err, "Handler failed with error");
            ErrorHandlingResult::unhandled()
        },
    }
}
