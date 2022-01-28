use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use indoc::formatdoc;
use rspotify::clients::OAuthClient;
use rspotify::model::{FullTrack, TrackId};
use rustrict::Type;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify::CurrentlyPlaying;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::{Status, TrackStatusService};
use crate::{genius, profanity, rickroll, spotify, state, telegram};

pub const CHECK_INTERVAL: u64 = 2;

async fn check_bad_words(state: &state::UserState, track: &FullTrack) -> anyhow::Result<()> {
    let Some(hit) = genius::search_for_track(state, track).await? else {
        return Ok(());
    };

    let lyrics = genius::get_lyrics(&hit).await?;

    let check = profanity::Manager::check(lyrics);

    if !check.should_trigger() {
        return Ok(());
    }

    let bad_lines: Vec<_> = check
        .into_iter()
        .filter(|profanity::LineResult { typ, .. }| !typ.is(Type::SAFE))
        .map(|line: profanity::LineResult| {
            format!(
                "`{}:` {}, `[{}]`",
                line.no + 1,
                line.highlighted(),
                line.get_type_name()
            )
        })
        .collect();

    if bad_lines.is_empty() {
        return Ok(());
    }

    let mut lines = bad_lines.len();
    let message = loop {
        let message = formatdoc!(
            // TODO Return spoilers after teloxide update
            // "has bad words: \n ||{}||",
            "
                Current song \\({track_name}\\) probably has bad words \\(ignore in case of false positive\\):
                
                {bad_lines}
                
                [Genius Source]({genius_link})
            ",
            track_name = spotify::create_track_name(track),
            bad_lines = bad_lines[0..lines].join("\n"),
            genius_link = hit
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

async fn check_playing_for_user(
    app_state: &'static state::AppState,
    user_id: &str,
    prevs: &mut HashMap<String, TrackId>,
) -> anyhow::Result<String> {
    let state = app_state
        .user_state(user_id)
        .await
        .context("Get user state")?;

    if rickroll::should(&state.user_id, false).await {
        rickroll::like(&state).await;
    }

    let playing = spotify::currently_playing(&*state.spotify.read().await).await;

    let track = match playing {
        CurrentlyPlaying::Err(err) => {
            return Err(err).context("Get currently playing track");
        }
        CurrentlyPlaying::None(message) => {
            return Ok(message);
        }
        CurrentlyPlaying::Ok(track) => track,
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
            state
                .spotify
                .read()
                .await
                .next_track(None)
                .await
                .context("Skip current track")?;
        }
        Status::None => {
            if prevs.get(user_id) == track.id.as_ref() {
                return Ok("Skip same track".to_owned());
            }

            check_bad_words(&state, &track)
                .await
                .context("Check bad words")?;
        }
        Status::Ignore => {}
    }

    if let Some(id) = track.id {
        prevs.insert(user_id.to_owned(), id);
    }

    Ok("Complete check".to_owned())
}

pub async fn check_playing(app_state: &'static state::AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(CHECK_INTERVAL));
    let mut prevs: HashMap<String, TrackId> = HashMap::new();
    loop {
        interval.tick().await;

        let user_ids = match SpotifyAuthService::get_registered(&app_state.db).await {
            Ok(user_ids) => user_ids,
            Err(err) => {
                tracing::error!("Something went wrong: {:?}", err);
                continue;
            }
        };

        for user_id in user_ids {
            let user_id = user_id.as_str();
            if let Err(err) = check_playing_for_user(app_state, user_id, &mut prevs).await {
                tracing::error!(user_id, "Something went wrong: {:?}", err);
            }
        }
    }
}
