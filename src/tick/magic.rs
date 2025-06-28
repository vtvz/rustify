use rspotify::clients::OAuthClient;
use rspotify::model::{Context as SpotifyContext, Id, PlaylistId, Type as SpotifyType};

use crate::app::App;
use crate::entity::prelude::UserModel;
use crate::magic_service::MagicService;
use crate::spotify::ShortTrack;
use crate::user::UserState;

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
    user: &UserModel,
) -> anyhow::Result<()> {
    let Some(context) = context else {
        return Ok(());
    };

    if context._type != SpotifyType::Playlist {
        return Ok(());
    }

    let Some(playlist_id) = user.magic_playlist.as_ref() else {
        return Ok(());
    };

    let playlist_id = PlaylistId::from_id(playlist_id)?;

    if playlist_id.uri() != context.uri {
        return Ok(());
    }

    let mut redis_conn = app.redis_conn().await?;

    let already_removed =
        MagicService::is_already_removed(&mut redis_conn, state.user_id(), track.id()).await?;

    if already_removed {
        return Ok(());
    }

    state
        .spotify()
        .await
        .playlist_remove_all_occurrences_of_items(
            playlist_id,
            Some(track.raw_id().to_owned().into()),
            None,
        )
        .await?;

    MagicService::set_already_removed(&mut redis_conn, state.user_id(), track.id()).await?;

    Ok(())
}
