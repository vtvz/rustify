use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use indoc::formatdoc;
use rspotify::clients::OAuthClient;
use rspotify::model::{
    Context as SpotifyContext,
    FullTrack,
    PlayableId,
    PlaylistId,
    TrackId,
    Type as SpotifyType,
};
use rustrict::Type;
use teloxide::prelude2::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};
use tokio::sync::{RwLock, Semaphore};

use crate::spotify::CurrentlyPlaying;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::{Status, TrackStatusService};
use crate::{profanity, rickroll, spotify, state, telegram};

pub const CHECK_INTERVAL: u64 = 3;
const PARALLEL_CHECKS: usize = 1;

type PrevTracksMap = Arc<RwLock<HashMap<String, TrackId>>>;

async fn check_bad_words(state: &state::UserState, track: &FullTrack) -> anyhow::Result<()> {
    let Some(hit) = state.app.lyrics.search_for_track(track).await? else {
        return Ok(());
    };

    if hit.language() != "en" {
        tracing::trace!("Track has non English lyrics: {:?}", track.id);

        return Ok(());
    }

    let check = profanity::Manager::check(hit.lyrics());

    if !check.should_trigger() {
        return Ok(());
    }

    let bad_lines: Vec<_> = check
        .into_iter()
        .filter(|profanity::LineResult { typ, .. }| !typ.is(Type::SAFE))
        .map(|line: profanity::LineResult| {
            format!(
                "`{}:` {}, `[{}]`",
                hit.line_index_name(line.no),
                line.highlighted(),
                line.typ
            )
        })
        .collect();

    if bad_lines.is_empty() {
        return Ok(());
    }

    let mut lines = bad_lines.len();
    let message = loop {
        let message = formatdoc!(
            "
                Current song \\({track_name}\\) probably has bad words \\(ignore in case of false positive\\):
                
                {bad_lines}
                
                {genius}
            ",
            track_name = spotify::create_track_name(track),
            bad_lines = bad_lines[0..lines].join("\n"),
            genius = hit.tg_link(true)
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    state
        .app
        .bot
        .send_message(state.user_id.clone(), message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            #[rustfmt::skip]
            vec![
                vec![InlineButtons::Dislike(spotify::get_track_id(track)).into()],
                vec![InlineButtons::Ignore(spotify::get_track_id(track)).into()],
            ],
        )))
        .send()
        .await?;

    Ok(())
}

async fn handle_disliked_track(
    state: &state::UserState,
    track: &FullTrack,
    context: Option<&SpotifyContext>,
) -> anyhow::Result<()> {
    if state.is_spotify_premium() {
        let spotify = state.spotify.read().await;

        spotify
            .next_track(None)
            .await
            .context("Skip current track")?;

        let track_id = spotify::get_track_id(track);
        TrackStatusService::increase_skips(&state.app.db, &state.user_id, &track_id).await?;

        let Some(context) = context else {
            return Ok(());
        };

        match context._type {
            SpotifyType::Playlist => {
                let track_id = TrackId::from_str(&track_id)?;
                let hate: Option<&dyn PlayableId> = Some(&track_id);

                spotify
                    .playlist_remove_all_occurrences_of_items(
                        &PlaylistId::from_str(&context.uri)?,
                        hate,
                        None,
                    )
                    .await?;
            }

            SpotifyType::Collection => {
                let track_id = TrackId::from_str(&track_id)?;

                spotify
                    .current_user_saved_tracks_delete(Some(&track_id))
                    .await?;
            }
            _ => {}
        }

        return Ok(());
    }

    let message = format!(
        "Current song \\({track_name}\\) was disliked, but I cannot skip it...",
        track_name = spotify::create_track_name(track),
    );

    state
        .app
        .bot
        .send_message(state.user_id.clone(), message)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(())
}

async fn check_playing_for_user(
    app_state: &'static state::AppState,
    user_id: &str,
    prevs: PrevTracksMap,
) -> anyhow::Result<String> {
    let state = app_state
        .user_state(user_id)
        .await
        .context("Get user state")?;

    if rickroll::should(&state.user_id, false).await {
        rickroll::like(&state).await;
    }

    let playing = spotify::currently_playing(&*state.spotify.read().await).await;

    let (track, context) = match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        }
        CurrentlyPlaying::None(message) => {
            return Ok(message);
        }
        CurrentlyPlaying::Ok(track, context) => (track, context),
    };

    if rickroll::should(&state.user_id, true).await {
        rickroll::queue(&state).await;
    }

    let status = TrackStatusService::get_status(
        &state.app.db,
        &state.user_id,
        &spotify::get_track_id(&track),
    )
    .await;

    match status {
        Status::Disliked => {
            handle_disliked_track(&state, &track, context.as_ref())
                .await
                .context("Handle Disliked Tracks")?;
        }
        Status::None => {
            if prevs.read().await.get(user_id) == track.id.as_ref() {
                return Ok("Skip same track".to_owned());
            }

            check_bad_words(&state, &track)
                .await
                .context("Check bad words")?;
        }
        Status::Ignore => {}
    }

    if let Some(id) = track.id {
        prevs.write().await.insert(user_id.to_owned(), id);
    }

    Ok("Complete check".to_owned())
}

pub async fn check_playing(app_state: &'static state::AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(CHECK_INTERVAL));
    let prevs: PrevTracksMap = Arc::new(RwLock::new(HashMap::new()));
    loop {
        interval.tick().await;

        let user_ids = match SpotifyAuthService::get_registered(&app_state.db).await {
            Ok(user_ids) => user_ids,
            Err(err) => {
                tracing::error!("Something went wrong: {:?}", err);
                continue;
            }
        };

        let semaphore = Arc::new(Semaphore::new(PARALLEL_CHECKS));
        let mut join_handles = Vec::new();

        for user_id in user_ids {
            let prevs = Arc::clone(&prevs);

            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .context("Shouldn't fail")
                .expect("Shouldn't fail");

            join_handles.push(tokio::spawn(async move {
                let user_id = user_id.as_str();
                if let Err(err) = check_playing_for_user(app_state, user_id, prevs).await {
                    tracing::error!(user_id, "Something went wrong: {:?}", err);
                }
                drop(permit);
            }));
        }

        for handle in join_handles {
            handle
                .await
                .context("Shouldn't fail")
                .expect("Shouldn't fail");
        }
    }
}
