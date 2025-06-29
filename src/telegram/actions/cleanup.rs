use anyhow::Context;
use rspotify::DEFAULT_PAGINATION_CHUNKS;
use rspotify::clients::OAuthClient;
use rspotify::model::{Page, PlayableId};
use teloxide::prelude::*;
use teloxide::types::ReplyParameters;

use crate::app::App;
use crate::entity::prelude::*;
use crate::telegram::handlers::HandleStatus;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;
use crate::user_service::UserService;
use crate::utils::retry;

pub async fn handle(
    app: &'static App,
    state: &UserState,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let message = app
        .bot()
        .send_message(m.chat.id, t!("dump.cleanup-start", locale = state.locale()))
        .reply_parameters(ReplyParameters::new(m.id))
        .await?;

    let spotify = state.spotify().await;

    let me = state
        .spotify_user()
        .await?
        .context("Spotify user not found")?;

    let disliked =
        TrackStatusService::get_ids_with_status(app.db(), state.user_id(), TrackStatus::Disliked)
            .await?;

    let Page {
        total: liked_before,
        ..
    } = retry(|| spotify.current_user_saved_tracks_manual(None, Some(1), None)).await?;

    for chunk in disliked.chunks(50) {
        retry(|| spotify.current_user_saved_tracks_delete(chunk.iter().cloned()))
            .await
            .context("Cannot remove occurrences of items for saved songs")?;
    }

    let Page {
        total: liked_after, ..
    } = retry(|| spotify.current_user_saved_tracks_manual(None, Some(1), None)).await?;

    let mut offset = 0;
    let mut before = 0;
    let mut count = 0u32;

    // current_user_playlists for some reason has the issue with Send
    loop {
        let Page {
            items: playlists,
            next,
            ..
        } = retry(|| {
            spotify.current_user_playlists_manual(Some(DEFAULT_PAGINATION_CHUNKS), Some(offset))
        })
        .await
        .context("Cannot get current user playlists")?;

        offset += playlists.len() as u32;

        for playlist in playlists {
            if playlist.owner.id != me.id {
                continue;
            }
            count += 1;

            before += playlist.tracks.total;

            // let chunks: Vec<Vec<TrackId>> = ;

            for chunk in disliked.chunks(100) {
                retry(|| {
                    let hate: Vec<PlayableId> =
                        chunk.iter().map(|item| item.clone().into()).collect();

                    spotify.playlist_remove_all_occurrences_of_items(
                        playlist.id.clone(),
                        hate,
                        None,
                    )
                })
                .await
                .context(format!(
                    "Cannot remove occurrences of items for playlist {}",
                    playlist.id
                ))?;
            }
        }

        if next.is_none() {
            break;
        }
    }

    let mut offset = 0;
    let mut after = 0;
    loop {
        let Page {
            items: playlists,
            next,
            ..
        } = retry(|| {
            spotify.current_user_playlists_manual(Some(DEFAULT_PAGINATION_CHUNKS), Some(offset))
        })
        .await?;

        offset += playlists.len() as u32;

        for playlist in playlists {
            if playlist.owner.id != me.id {
                continue;
            }
            after += playlist.tracks.total;
        }

        if next.is_none() {
            break;
        }
    }

    let removed_playlists = before - after;
    let removed_collection = liked_before - liked_after;

    UserService::increase_stats_query(state.user_id())
        .removed_playlists(removed_playlists)
        .removed_collection(removed_collection)
        .exec(app.db())
        .await?;

    app.bot()
        .edit_message_text(
            message.chat.id,
            message.id,
            t!(
                "dump.cleanup-finish",
                locale = state.locale(),
                removed_playlists = removed_playlists,
                count_playlists = count,
                removed_collection = removed_collection
            ),
        )
        .await?;

    Ok(HandleStatus::Handled)
}
