use anyhow::Context;
use rspotify::clients::OAuthClient;
use rspotify::model::{
    Context as SpotifyContext,
    PlayableId,
    PlaylistId,
    TrackId,
    Type as SpotifyType,
};
use teloxide::prelude::*;
use teloxide::types::{ChatId, ParseMode};

use crate::spotify::ShortTrack;
use crate::state;
use crate::state::AppState;
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;

#[tracing::instrument(
    skip_all,
    fields(
        track_id = track.id(),
        track_name = track.name_with_artists(),
    )
)]
pub async fn handle(
    app_state: &'static AppState,
    state: &state::UserState,
    track: &ShortTrack,
    context: Option<&SpotifyContext>,
) -> anyhow::Result<()> {
    if state.is_spotify_premium().await? {
        let spotify = state.spotify().await;

        spotify
            .next_track(None)
            .await
            .context("Skip current track")?;

        let track_id = track.id();
        TrackStatusService::increase_skips(app_state.db(), state.user_id(), track_id).await?;

        let Some(context) = context else {
            return Ok(());
        };

        match context._type {
            SpotifyType::Playlist => {
                let track_id = TrackId::from_id(track_id)?;
                let hate: Option<PlayableId> = Some(track_id.into());

                let res = spotify
                    .playlist_remove_all_occurrences_of_items(
                        PlaylistId::from_id_or_uri(&context.uri)?,
                        hate,
                        None,
                    )
                    .await;

                // It's a bit too much to check if user owns this playlist
                if res.is_ok() {
                    UserService::increase_stats_query(state.user_id())
                        .removed_playlists(1)
                        .exec(app_state.db())
                        .await?;
                }
            },

            SpotifyType::Collection => {
                let track_id = TrackId::from_id(track_id)?;

                spotify
                    .current_user_saved_tracks_delete(Some(track_id))
                    .await?;

                UserService::increase_stats_query(state.user_id())
                    .removed_collection(1)
                    .exec(app_state.db())
                    .await?;
            },
            _ => {},
        }

        return Ok(());
    }

    let message = format!(
        "Current song ({track_name}) was disliked, but I cannot skip it...",
        track_name = track.track_tg_link(),
    );

    let result = app_state
        .bot()
        .send_message(ChatId(state.user_id().parse()?), message)
        .parse_mode(ParseMode::Html)
        .send()
        .await;

    super::errors::telegram(app_state, state, result)
        .await
        .map(|_| ())
}
