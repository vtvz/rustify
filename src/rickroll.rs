use chrono::Duration;
use deadpool_redis::redis::AsyncCommands;
use rspotify::model::TrackId;
use rspotify::prelude::OAuthClient;

use crate::app::App;
use crate::user::UserState;

#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn queue(app: &'static App, state: &UserState) -> anyhow::Result<()> {
    let key = format!("rustify:rickroll:{user_id}", user_id = state.user_id());
    let mut redis = app.redis_conn().await?;
    let ttl = Duration::days(30).num_seconds() as u64;

    let rickrolled: bool = redis.exists(&key).await?;

    if rickrolled {
        return Ok(());
    }

    state
        .spotify()
        .await
        .add_item_to_queue(TrackId::from_id("4PTG3Z6ehGkBFwjybzWkR8")?.into(), None)
        .await?;

    tracing::info!(user_id = state.user_id(), "The victim of Rickroll");

    let _: () = redis.set_ex(key, true, ttl).await?;

    Ok(())
}
