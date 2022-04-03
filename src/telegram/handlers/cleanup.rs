use anyhow::Context;
use rspotify::clients::OAuthClient;
use rspotify::model::{Page, PlayableId};
use rspotify::DEFAULT_PAGINATION_CHUNKS;
use teloxide::prelude2::*;

use crate::state::UserState;
use crate::track_status_service::{Status, TrackStatusService};

pub async fn handle(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let message = bot
        .send_message(
            m.chat.id,
            "Started cleanup. Please wait, it can take a bit of time 🕐",
        )
        .send()
        .await?;

    let spotify = state.spotify.read().await;
    let me = spotify
        .current_user()
        .await
        .context("Cannot get current user")?;

    let disliked =
        TrackStatusService::get_ids_with_status(&state.app.db, &state.user_id, Status::Disliked)
            .await?;

    let Page {
        total: liked_before,
        ..
    } = spotify
        .current_user_saved_tracks_manual(None, Some(1), None)
        .await?;

    for chunk in disliked.chunks(50) {
        spotify
            .current_user_saved_tracks_delete(chunk)
            .await
            .context("Cannot remove occurrences of items for saved songs")?;
    }

    let Page {
        total: liked_after, ..
    } = spotify
        .current_user_saved_tracks_manual(None, Some(1), None)
        .await?;

    let mut offset = 0;
    let mut before = 0;
    let mut count = 0u32;

    // current_user_playlists for some reason has the issue with Send
    loop {
        let Page {
            items: playlists,
            next,
            ..
        } = spotify
            .current_user_playlists_manual(Some(DEFAULT_PAGINATION_CHUNKS), Some(offset))
            .await
            .context("Cannot get current user playlists")?;

        offset += playlists.len() as u32;

        for playlist in playlists {
            if playlist.owner.id != me.id {
                continue;
            }
            count += 1;

            before += playlist.tracks.total;

            for chunk in disliked.chunks(100) {
                let hate: Vec<&dyn PlayableId> =
                    chunk.iter().map(|item| item as &dyn PlayableId).collect();
                spotify
                    .playlist_remove_all_occurrences_of_items(&playlist.id, hate, None)
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
        } = spotify
            .current_user_playlists_manual(Some(DEFAULT_PAGINATION_CHUNKS), Some(offset))
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
    bot.edit_message_text(
        message.chat.id,
        message.id,
        format!(
            "Deleted {} tracks in {} playlists and {} in favorite songs 🗑",
            before - after,
            count,
            liked_before - liked_after
        ),
    )
    .send()
    .await?;

    Ok(true)
}
