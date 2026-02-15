// use anyhow::Context;
use reqwest::StatusCode;
use teloxide::ApiError;
use teloxide::payloads::SendMessageSetters as _;
use teloxide::prelude::Requester as _;
use teloxide::types::ChatId;

use crate::app::App;
use crate::entity::prelude::UserStatus;
use crate::services::{MetricsService, UserService};
use crate::spotify;
// use crate::spotify::auth::SpotifyAuthService;
use crate::telegram::commands::UserCommandDisplay;
use crate::telegram::keyboards::StartKeyboard;

#[derive(Default)]
pub struct ErrorHandlingResult {
    pub handled: bool,
    pub user_notified: bool,
}

impl ErrorHandlingResult {
    #[must_use]
    pub fn unhandled() -> Self {
        Self {
            handled: false,
            user_notified: false,
        }
    }

    #[must_use]
    pub fn handled() -> Self {
        Self {
            handled: true,
            user_notified: false,
        }
    }

    #[must_use]
    pub fn handled_notified() -> Self {
        Self {
            handled: true,
            user_notified: true,
        }
    }
}

#[tracing::instrument(skip_all, fields(%user_id))]
pub async fn handle_blocked_bot(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let Some(err) = err.downcast_mut::<teloxide::RequestError>() else {
        return Ok(ErrorHandlingResult::unhandled());
    };

    if matches!(err, teloxide::RequestError::Api(ApiError::BotBlocked)) {
        UserService::set_status(app.db(), user_id, UserStatus::BotBlocked).await?;

        return Ok(ErrorHandlingResult::handled_notified());
    }

    return Ok(ErrorHandlingResult::unhandled());
}

#[tracing::instrument(skip_all, fields(%user_id))]
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

        MetricsService::spotify_429_inc(&mut app.redis_conn().await?).await?;

        /*
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
        */
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

                        UserService::set_status(app.db(), user_id, UserStatus::SpotifyForbidden)
                            .await?;

                        app.bot()
                            .send_message(
                                ChatId(user_id.parse()?),
                                t!(
                                    "error.unavailable-in-country",
                                    locale = locale,
                                    command = UserCommandDisplay::Login,
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

                    UserService::set_status(app.db(), user_id, UserStatus::SpotifyTokenInvalid)
                        .await?;

                    app.bot()
                        .send_message(
                            ChatId(user_id.parse()?),
                            t!(
                                "error.spotify-auth-failed",
                                locale = locale,
                                command = UserCommandDisplay::Login,
                                error = serr.error_description,
                            ),
                        )
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

#[tracing::instrument(skip_all, fields(%user_id))]
pub async fn spotify_client_error(
    err: &mut anyhow::Error,
    app: &App,
    user_id: &str,
    locale: &str,
) -> anyhow::Result<ErrorHandlingResult> {
    let Some(err) = err.downcast_mut::<rspotify::ClientError>() else {
        return Ok(ErrorHandlingResult::unhandled());
    };

    if matches!(err, rspotify::ClientError::InvalidToken) {
        tracing::error!(err = ?err, user_id, "User has Invalid Spotify Token");
        UserService::set_status(app.db(), user_id, UserStatus::SpotifyTokenInvalid).await?;

        app.bot()
            .send_message(
                ChatId(user_id.parse()?),
                t!(
                    "error.spotify-invalid-token",
                    locale = locale,
                    command = UserCommandDisplay::Login
                ),
            )
            .reply_markup(StartKeyboard::markup(locale))
            .await?;

        return Ok(ErrorHandlingResult::handled_notified());
    }

    tracing::error!(err = ?err, "Spotify Client Has an Issue");

    Ok(ErrorHandlingResult::handled())
}

#[tracing::instrument(skip_all, fields(%user_id))]
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

#[tracing::instrument(skip_all, fields(%user_id))]
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
