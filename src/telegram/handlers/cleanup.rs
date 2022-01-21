use anyhow::Context;
use rspotify::clients::OAuthClient;
use rspotify::model::{Page, PlayableId};
use rspotify::DEFAULT_PAGINATION_CHUNKS;
use teloxide::prelude::*;

use crate::state::UserState;
use crate::{Status, TrackStatusService};

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> anyhow::Result<bool> {
    let message = cx
        .answer("Started cleanup. Please wait, it can take a bit of time 🕐")
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

    let mut offset = 0;
    let mut before = 0;
    let mut count = 0u32;

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
    cx.requester
        .edit_message_text(
            message.chat_id(),
            message.id,
            format!("Deleted {} tracks in {} playlists 🗑", before - after, count),
        )
        .send()
        .await?;

    Ok(true)
}
