use chrono::Duration;
use redis::AsyncCommands;
use rspotify::clients::OAuthClient;
use rspotify::model::{Context as SpotifyContext, Id, PlaylistId, Type as SpotifyType};

use crate::app::App;
use crate::spotify::ShortTrack;
use crate::user::UserState;
use crate::user_service::UserService;

#[tracing::instrument(
    skip_all,
    fields(
        track_id = track.id(),
        track_name = track.name_with_artists(),
    )
)]
pub async fn handle(
    app: &'static App,
    state: &UserState,
    track: &ShortTrack,
    context: Option<&SpotifyContext>,
) -> anyhow::Result<()> {
    let Some(context) = context else {
        return Ok(());
    };

    if context._type != SpotifyType::Playlist {
        return Ok(());
    }

    let key = format!(
        "rustify:magic:{user_id}:{track_id}",
        user_id = state.user_id(),
        track_id = track.id()
    );

    let mut redis = app.redis_conn().await?;
    let ttl = Duration::minutes(10).num_seconds() as u64;

    let already_removed: bool = redis.exists(&key).await?;

    if already_removed {
        return Ok(());
    }

    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;

    let Some(playlist_id) = user.magic_playlist else {
        return Ok(());
    };

    let playlist_id = PlaylistId::from_id(playlist_id)?;

    if playlist_id.uri() != context.uri {
        return Ok(());
    }

    state
        .spotify()
        .await
        .playlist_remove_all_occurrences_of_items(
            PlaylistId::from_id_or_uri(&context.uri)?,
            Some(track.raw_id().to_owned().into()),
            None,
        )
        .await?;

    let _: () = redis.set_ex(key, true, ttl).await?;

    Ok(())
}
