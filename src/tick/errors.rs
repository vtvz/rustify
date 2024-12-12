use anyhow::Context;
use reqwest::{Response, StatusCode};
use sea_orm::DbConn;
use teloxide::ApiError;
use teloxide::prelude::*;

use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state;
use crate::state::AppState;
use crate::user_service::UserService;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn telegram(
    app: &'static AppState,
    state: &state::UserState,
    result: Result<Message, teloxide::RequestError>,
) -> anyhow::Result<Message> {
    if let Err(teloxide::RequestError::Api(ApiError::BotBlocked | ApiError::InvalidToken)) = result
    {
        UserService::set_status(app.db(), state.user_id(), UserStatus::Blocked).await?;
    }

    result.map_err(|err| err.into())
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn handle_too_many_requests(
    db: &DbConn,
    user_id: &str,
    response: &Response,
) -> anyhow::Result<()> {
    if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return Ok(());
    }

    tracing::info!("User got a 429 error (too many requests)");

    let header = response
        .headers()
        .get("Retry-After")
        .context("Need Retry-After header to proceed")?;

    let retry_after: i64 = header.to_str()?.parse()?;

    SpotifyAuthService::suspend_for(db, &[user_id], chrono::Duration::seconds(retry_after)).await?;

    Ok(())
}
