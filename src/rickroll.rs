use anyhow::Context;
use chrono::Duration;
use redis::AsyncCommands;
use rspotify::model::TrackId;
use rspotify::prelude::OAuthClient;

use crate::app::App;

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn queue(app: &'static App, user_id: &str) -> anyhow::Result<()> {
    let state = app.user_state(user_id).await.context("Get user state")?;

    let key = format!("rustify:rickroll:{user_id}");
    let mut redis = app.redis_conn().await?;
    let ttl = Duration::days(2).num_seconds() as u64;

    let to_rickroll: u8 = redis.exists(&key).await?;

    if to_rickroll != 0 {
        return Ok(());
    }

    tracing::debug!(user_id = user_id, "The victim of Rickroll");

    state
        .spotify()
        .await
        .add_item_to_queue(
            rspotify::model::PlayableId::Track(TrackId::from_id("4PTG3Z6ehGkBFwjybzWkR8")?),
            None,
        )
        .await?;

    let _: () = redis.set_ex(key, true, ttl).await?;

    Ok(())
}
