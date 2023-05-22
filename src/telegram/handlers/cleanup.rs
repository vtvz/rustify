use rspotify::clients::OAuthClient;
use rspotify::model::{Page, PlayableId};
use rspotify::DEFAULT_PAGINATION_CHUNKS;
use teloxide::prelude::*;

use crate::entity::prelude::*;
use crate::errors::{Context, GenericResult};
use crate::state::UserState;
use crate::track_status_service::TrackStatusService;
use crate::user_service::UserService;
use crate::utils::retry;

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> GenericResult<bool> {
    let message = bot
        .send_message(
            m.chat.id,
            "Started cleanup. Please wait, it can take a bit of time 🕐",
        )
        .reply_to_message_id(m.id)
        .send()
        .await?;

    let spotify = state.spotify.read().await;

    let me = state
        .spotify_user()
        .await?
        .context("Spotify user not found")?;

    let disliked = TrackStatusService::get_ids_with_status(
        &state.app.db,
        &state.user_id,
        TrackStatus::Disliked,
    )
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
                    let hate: Vec<PlayableId> = chunk
                        .iter()
                        .map(|item| PlayableId::Track(item.clone()))
                        .collect();

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

    UserService::increase_stats_query(&state.user_id)
        .removed_playlists(removed_playlists)
        .removed_collection(removed_collection)
        .exec(&state.app.db)
        .await?;

    bot.edit_message_text(
        message.chat.id,
        message.id,
        format!(
            "Deleted {} tracks in {} playlists and {} in favorite songs 🗑",
            removed_playlists, count, removed_collection
        ),
    )
    .send()
    .await?;

    Ok(true)
}
