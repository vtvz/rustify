use anyhow::Context;
use rspotify::clients::OAuthClient;

use crate::app::App;
use crate::skippage_service::SkippageService;
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
) -> anyhow::Result<bool> {
    if state.user().cfg_skippage_secs == 0 {
        return Ok(false);
    }

    let mut redis_conn = app.redis_conn().await?;

    let current_playing =
        SkippageService::get_current_playing(&mut redis_conn, state.user_id()).await?;

    if current_playing == track.id() {
        return Ok(false);
    }

    let track_exists =
        SkippageService::get_track_played(&mut redis_conn, state.user_id(), track.id()).await?;

    if track_exists {
        let spotify = state.spotify().await;

        spotify
            .next_track(None)
            .await
            .context("Skip track in Spotify")?;
    }

    SkippageService::save_current_playing(&mut redis_conn, state.user_id(), track.id()).await?;
    SkippageService::save_track_played(
        &mut redis_conn,
        state.user_id(),
        track.id(),
        state.user().cfg_skippage_secs as u64,
    )
    .await?;

    Ok(true)
}
