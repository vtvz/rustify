use futures::StreamExt;
use indoc::formatdoc;
use rand::seq::SliceRandom;
use rspotify::model::{Id, UserId};
use rspotify::prelude::{BaseClient as _, OAuthClient as _};
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, ParseMode};

use crate::app::App;
use crate::spotify::ShortPlaylist;
use crate::telegram::actions;
use crate::telegram::handlers::HandleStatus;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::user_service::UserService;

async fn get_playlist(
    state: &UserState,
    spotify_user_id: UserId<'static>,
    magic_playlist_id: String,
) -> anyhow::Result<ShortPlaylist> {
    let playlist_name = "Magic✨";

    let spotify = state.spotify().await;

    let mut playlists_stream = spotify.user_playlists(spotify_user_id.clone());

    while let Some(playlist) = playlists_stream.next().await {
        let playlist = playlist?;
        if playlist.id.id() == magic_playlist_id {
            spotify
                .playlist_replace_items(playlist.id.clone(), [])
                .await?;

            return Ok(playlist.into());
        }
    }

    let playlist = spotify
        .user_playlist_create(
            spotify_user_id,
            playlist_name,
            Some(false),
            Some(false),
            Some("Autogenerated Playlist by Rustify Bot"),
        )
        .await?;

    Ok(playlist.into())
}

pub async fn handle(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    if !state.is_spotify_authed().await {
        actions::register::send_register_invite(app, chat_id).await?;

        return Ok(HandleStatus::Handled);
    }

    let Some(spotify_user) = state.spotify_user().await? else {
        return Ok(HandleStatus::Skipped);
    };

    let header = formatdoc!(
        "
            <i>Magic Playlist™</i> ✨ is made of your shuffled favorite songs that will be removed from this playlist as you listen to them. \
            This allows you to listen to everything you love one by one without any repetition. You'll love it! 😊"
    );

    let m = app
        .bot()
        .send_message(
            chat_id,
            format!("{header}\n⏳ Generating <i>Magic Playlist™</i> ✨"),
        )
        .parse_mode(ParseMode::Html)
        .await?;

    let spotify = state.spotify().await;
    let mut saved_tracks = spotify.current_user_saved_tracks(None);
    let mut track_ids = vec![];
    while let Some(track) = saved_tracks.next().await {
        let track = track?;
        if let Some(track_id) = track.track.id {
            track_ids.push(track_id.into())
        }
    }

    track_ids.shuffle(&mut rand::rng());

    let user = UserService::obtain_by_id(app.db(), state.user_id()).await?;
    let playlist = get_playlist(
        state,
        spotify_user.id,
        user.magic_playlist.unwrap_or("none".into()),
    )
    .await?;

    UserService::set_magic_playlist(app.db(), state.user_id(), playlist.id().id()).await?;

    for chunk in track_ids.chunks(100) {
        spotify
            .playlist_add_items(playlist.id().clone(), chunk.iter().cloned(), None)
            .await?;
    }

    app.bot()
        .edit_message_text(
            m.chat.id,
            m.id,
            formatdoc!(
                r#"
                    {header}

                    ✨ Created <a href="{}">Magic Playlist™</a> ✨"#,
                playlist.url()
            ),
        )
        .parse_mode(ParseMode::Html)
        .link_preview_options(link_preview_small_top(playlist.url()))
        .await?;

    Ok(HandleStatus::Handled)
}
