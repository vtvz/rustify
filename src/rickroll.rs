use anyhow::Context;
use chrono::Duration;
use redis::AsyncCommands;
use rspotify::model::TrackId;
use rspotify::prelude::OAuthClient;

use crate::app::App;
use crate::spotify::CurrentlyPlaying;

#[tracing::instrument(skip_all, fields(user_id = %user_id))]
pub async fn queue(app: &'static App, user_id: &str) -> anyhow::Result<()> {
    let state = app.user_state(user_id).await.context("Get user state")?;

    let playing = CurrentlyPlaying::get(&*state.spotify().await).await;

    match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        },
        CurrentlyPlaying::None(_) => {
            return Ok(());
        },
        CurrentlyPlaying::Ok(..) => (),
    };

    let key = format!("rustify:rickroll:{user_id}");
    let mut redis = app.redis_conn().await?;
    let ttl = Duration::days(2).num_seconds() as u64;

    let rickrolled: bool = redis.exists(&key).await?;

    if rickrolled {
        return Ok(());
    }

    state
        .spotify()
        .await
        .add_item_to_queue(
            rspotify::model::PlayableId::Track(TrackId::from_id("4PTG3Z6ehGkBFwjybzWkR8")?),
            None,
        )
        .await?;

    tracing::debug!(user_id = user_id, "The victim of Rickroll");

    let _: () = redis.set_ex(key, true, ttl).await?;

    Ok(())
}
