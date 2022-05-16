use reqwest::{Response, StatusCode};
use sea_orm::DbConn;
use teloxide::prelude::*;
use teloxide::ApiError;

use crate::entity::prelude::*;
use crate::errors::{Context, GenericResult};
use crate::spotify_auth_service::SpotifyAuthService;
use crate::state;
use crate::user_service::UserService;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id))]
pub async fn telegram(
    state: &state::UserState,
    result: Result<Message, teloxide::RequestError>,
) -> GenericResult<Message> {
    if let Err(teloxide::RequestError::Api(ApiError::BotBlocked | ApiError::NotFound)) = result {
        UserService::set_status(&state.app.db, &state.user_id, UserStatus::Blocked).await?;
    }

    result.map_err(|err| err.into())
}

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn handle_too_many_requests(
    db: &DbConn,
    user_id: &str,
    response: &Response,
) -> GenericResult<()> {
    if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return Ok(());
    }

    tracing::info!("User got a 429 error (too many requests)");

    let header = response
        .headers()
        .get("Retry-After")
        .context("Need Retry-After header to proceed")?;

    let retry_after: i64 = header.to_str()?.parse()?;

    SpotifyAuthService::suspend_for(db, user_id, chrono::Duration::seconds(retry_after)).await?;

    Ok(())
}
