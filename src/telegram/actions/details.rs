use std::collections::HashSet;

use anyhow::anyhow;
use convert_case::{Case, Casing};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::clients::BaseClient as _;
use rspotify::model::{Modality, TrackId};
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt as _;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{
    RateLimitAction,
    RateLimitOutput,
    RateLimitService,
    TrackStatusService,
    WordStatsService,
};
use crate::spotify::{CurrentlyPlaying, ShortTrack};
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::user::UserState;
use crate::utils::{DurationPrettyFormat as _, StringUtils};
use crate::{profanity, telegram};

/// Handle the user's currently playing Spotify track and present its details to the chat.
///
/// Performs rate-limit enforcement for the "Details" action, fetches the user's currently
/// playing track from their Spotify context, and delegates presentation to the common handler.
/// If the user is rate-limited or nothing is playing, a localized message is sent and the
/// function returns without further processing.
///
/// # Returns
///
/// `Ok(HandleStatus::Handled)` when the request was processed (including early exit due to rate
/// limiting or no currently playing track), or an `Err` if an underlying service call fails.
///
/// # Examples
///
/// ```
/// # use crate::{App, UserState, ChatId, HandleStatus};
/// # async fn example(app: &'static App, state: &UserState, chat_id: ChatId) -> anyhow::Result<()> {
/// let status = crate::telegram::actions::details::handle_current(app, state, chat_id).await?;
/// match status {
///     HandleStatus::Handled => println!("Handled"),
///     HandleStatus::Skipped => println!("Skipped"),
/// }
/// # Ok(()) }
/// ```
#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_current(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
) -> anyhow::Result<HandleStatus> {
    let mut redis_conn = app.redis_conn().await?;
    if let RateLimitOutput::NeedToWait(duration) =
        RateLimitService::enforce_limit(&mut redis_conn, state.user_id(), RateLimitAction::Details)
            .await?
    {
        app.bot()
            .send_message(
                chat_id,
                t!(
                    "rate-limit.details",
                    duration = duration.pretty_format(),
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let spotify = state.spotify().await;
    let track = match spotify.current_playing_wrapped().await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(reason) => {
            app.bot()
                .send_message(chat_id, reason.localize(state.locale()))
                .await?;

            return Ok(HandleStatus::Handled);
        },
        CurrentlyPlaying::Ok(track, _) => *track,
    };

    common(app, state, chat_id, track).await
}

fn extract_id(url: &url::Url) -> Option<TrackId<'static>> {
    lazy_static! {
        static ref RE: Regex = Regex::new("^/track/([a-zA-Z0-9]+)$").expect("Should be compilable");
    }

    let cap = RE.captures(url.path())?;

    let id = TrackId::from_id(cap[1].to_owned());

    id.ok()
}

/// Handle a Spotify track URL by fetching the track and presenting its details to the message chat.
///
/// This enforces the per-user "Details" rate limit (sending a localized rate-limit message and
/// returning `HandleStatus::Handled` when the caller must wait), attempts to extract a Spotify
/// track ID from `url` (returning `HandleStatus::Skipped` if extraction fails), fetches a cached
/// short track using the user's Spotify context, and delegates presentation to the shared `common`
/// handler. Errors from Redis, rate limiting, or Spotify operations are propagated.
///
/// # Arguments
///
/// * `url` - The Spotify URL to extract a track ID from.
/// * `m` - The incoming Telegram message; replies and edits are sent to `m.chat.id`.
///
/// # Returns
///
/// `HandleStatus` indicating whether the message was handled or skipped; underlying I/O or API
/// errors are returned as `Err`.
///
/// # Examples
///
/// ```
/// # async fn example_call(app: &'static crate::App, state: &crate::UserState, url: &url::Url, m: &teloxide::types::Message) {
/// let result = crate::telegram::actions::details::handle_url(app, state, url, m).await;
/// assert!(result.is_ok());
/// # }
/// ```
#[tracing::instrument(skip_all, fields(user_id = %state.user_id()))]
pub async fn handle_url(
    app: &'static App,
    state: &UserState,
    url: &url::Url,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let mut redis_conn = app.redis_conn().await?;

    if let RateLimitOutput::NeedToWait(duration) =
        RateLimitService::enforce_limit(&mut redis_conn, state.user_id(), RateLimitAction::Details)
            .await?
    {
        app.bot()
            .send_message(
                m.chat.id,
                t!(
                    "rate-limit.details",
                    duration = duration.pretty_format(),
                    locale = state.locale()
                ),
            )
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let Some(track_id) = extract_id(url) else {
        return Ok(HandleStatus::Skipped);
    };

    let track = state
        .spotify()
        .await
        .short_track_cached(&mut app.redis_conn().await?, track_id)
        .await?;

    common(app, state, m.chat.id, track).await
}

/// Composes and sends a detailed message about a Spotify track (features, genres, status and lyrics)
///
/// This function prepares an initial "collecting info" message, fetches track features, artist genres,
/// track status and counts, searches for lyrics, applies profanity checks and word-stat updates, builds
/// an interactive inline keyboard (including an analyze button when AI is available), and edits the message
/// to include either a "no lyrics" notice or the formatted lyrics (trimmed to fit Telegram limits).
///
/// On success the function returns a handled status indicating the message was sent/edited; errors from
/// Spotify, database, or other services are propagated.
///
/// # Examples
///
/// ```no_run
/// # use std::sync::Arc;
/// # async fn example() -> anyhow::Result<()> {
/// #     // `app`, `state`, `chat_id` and `track` must be provided from your application context.
/// #     let app: &'static crate::App = todo!();
/// #     let state: crate::UserState = todo!();
/// #     let chat_id: teloxide::types::ChatId = todo!();
/// #     let track: crate::spotify::ShortTrack = todo!();
/// #     // call the helper
///     let _ = crate::telegram::actions::details::common(app, &state, chat_id, track).await?;
/// #     Ok(())
/// # }
/// ```
#[tracing::instrument(skip_all, fields(user_id = %state.user_id(), track_id = track.id(), track_name = track.name_with_artists()))]
async fn common(
    app: &'static App,
    state: &UserState,
    chat_id: ChatId,
    track: ShortTrack,
) -> anyhow::Result<HandleStatus> {
    let message = app
        .bot()
        .send_message(
            chat_id,
            t!(
                "details.collecting-info",
                locale = state.locale(),
                track_name = track.track_tg_link(),
                album_name = track.album_tg_link(),
            ),
        )
        .link_preview_options(link_preview_small_top(track.url()))
        .parse_mode(ParseMode::Html)
        .await?;

    let spotify = state.spotify().await;

    let status = TrackStatusService::get_status(app.db(), state.user_id(), track.id()).await;

    let mut keyboard = InlineButtons::from_track_status(status, track.id(), state.locale());

    // NOTE: It works because I have old token I need to cherish
    #[allow(deprecated)]
    let features = spotify.track_features(track.raw_id().clone()).await?;

    let modality = match features.mode {
        Modality::Minor => t!("details.minor", locale = state.locale()),
        Modality::Major => t!("details.major", locale = state.locale()),
        Modality::NoResult => t!("details.no-result", locale = state.locale()),
    };

    let key = match features.key {
        0 => "C",
        1 => "C♯/D♭",
        2 => "D",
        3 => "D♯/E♭",
        4 => "E",
        5 => "F",
        6 => "F♯/G♭",
        7 => "G",
        8 => "G♯/A♭",
        9 => "A",
        10 => "A♯/B♭",
        11 => "B",
        _ => "Unknown",
    };

    let disliked_by =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, None, Some(track.id()))
            .await?;

    let ignored_by =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, None, Some(track.id()))
            .await?;

    let genres: HashSet<_> = {
        let artist_ids = track.artist_raw_ids();

        let artists = match spotify.artists(artist_ids.iter().cloned()).await {
            // HACK: 403 "Spotify is unavailable in this country" error
            Err(rspotify::ClientError::Http(box rspotify::http::HttpError::StatusCode(resp))) => {
                tracing::info!("Resp from artists fetching {:?}", resp.text().await);

                vec![]
            },
            Err(err) => {
                tracing::error!("Err from artists fetching {:?}", err);

                return Err(err.into());
            },
            Ok(artists) => artists,
        };

        let search_url = url::Url::parse("https://open.spotify.com/search")?;

        artists
            .iter()
            .flat_map(|artist| artist.genres.clone())
            .unique()
            .map(|genre| {
                let mut url = search_url.clone();
                url.path_segments_mut()
                    .expect("Infallible")
                    .push(&format!(r#"genre:"{genre}""#));

                (genre.to_case(Case::Title), url)
            })
            .map(|(genre, url)| {
                format!(
                    r#"<a href="{url}">{genre}</a>"#,
                    genre = teloxide::utils::html::escape(&genre)
                )
            })
            .collect()
    };

    let genres_line = if genres.is_empty() {
        "".into()
    } else {
        t!(
            "details.genres",
            genres = genres.iter().join(", "),
            locale = state.locale()
        )
    };

    let header = t!(
        "details.header",
        locale = state.locale(),
        key = key,
        modality = modality,
        tempo = features.tempo,
        acousticness = (features.acousticness * 100.0).round() as u64,
        danceability = (features.danceability * 100.0).round() as u64,
        energy = (features.energy * 100.0).round() as u64,
        instrumentalness = (features.instrumentalness * 100.0).round() as u64,
        liveness = (features.liveness * 100.0).round() as u64,
        speechiness = (features.speechiness * 100.0).round() as u64,
        valence = (features.valence * 100.0).round() as u64,
        track_name = track.track_tg_link(),
        album_name = track.album_tg_link(),
        disliked_by = disliked_by,
        ignored_by = ignored_by,
    );

    let Some(hit) = app.lyrics().search_for_track(&track).await? else {
        app.bot()
            .edit_text(
                &message,
                t!(
                    "details.no-lyrics",
                    locale = state.locale(),
                    header = header.trim(),
                    genres_line = genres_line,
                ),
            )
            .parse_mode(ParseMode::Html)
            .reply_markup(InlineKeyboardMarkup::new(keyboard))
            .link_preview_options(link_preview_small_top(track.url()))
            .await?;

        return Ok(HandleStatus::Handled);
    };

    let checked = profanity::Manager::check(&hit.lyrics());

    WordStatsService::increase_details_occurence(app.db(), &checked.get_profine_words()).await?;

    let lyrics: Vec<_> = checked.iter().map(|line| line.highlighted()).collect();

    let typ = checked.typ.to_string();

    let mut lines = lyrics.len();
    // This requires to fit lyrics to tg message
    let text = loop {
        if lines == 0 {
            return Err(anyhow!("Issues with lyrics"));
        }

        let message = t!(
            "details.with-lyrics",
            locale = state.locale(),
            header = header.trim(),
            profanity = typ,
            language = hit.language(),
            genres_line = genres_line,
            lyrics = &lyrics[0..lines].join("\n"),
            lyrics_link = hit.link(),
            lyrics_link_text = hit.link_text(lines == lyrics.len()),
        );

        if message.chars_len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    if app.ai().is_some() {
        keyboard.push(vec![
            InlineButtons::Analyze(track.id().to_owned())
                .into_inline_keyboard_button(state.locale()),
        ]);
    }

    app.bot()
        .edit_text(&message, text)
        .parse_mode(ParseMode::Html)
        .reply_markup(InlineKeyboardMarkup::new(keyboard))
        .link_preview_options(link_preview_small_top(track.url()))
        .await?;

    Ok(HandleStatus::Handled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_id_success() {
        struct TestCase {
            url: &'static str,
            expected_id: &'static str,
            description: &'static str,
        }

        let test_cases = [
            TestCase {
                url: "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8?si=b248017abca04ef0",
                expected_id: "4PTG3Z6ehGkBFwjybzWkR8",
                description: "with query params",
            },
            TestCase {
                url: "https://open.spotify.com/track/abc123DEF456",
                expected_id: "abc123DEF456",
                description: "without query params",
            },
            TestCase {
                url: "https://open.spotify.com/track/AbC123DeF456",
                expected_id: "AbC123DeF456",
                description: "mixed case",
            },
            TestCase {
                url: "https://open.spotify.com/track/123456789",
                expected_id: "123456789",
                description: "all numeric",
            },
            TestCase {
                url: "https://open.spotify.com/track/abcdefghijk",
                expected_id: "abcdefghijk",
                description: "all letters",
            },
            TestCase {
                url: "https://spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8",
                expected_id: "4PTG3Z6ehGkBFwjybzWkR8",
                description: "different domain",
            },
        ];

        for tc in test_cases {
            let url = url::Url::parse(tc.url).unwrap();
            let id = extract_id(&url);
            assert_eq!(
                id,
                Some(TrackId::from_id(tc.expected_id).unwrap()),
                "failed for: {}",
                tc.description
            );
        }
    }

    #[test]
    fn extract_id_failure() {
        struct TestCase {
            url: &'static str,
            description: &'static str,
        }

        let test_cases = [
            TestCase {
                url: "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8_?si=b248017abca04ef0",
                description: "invalid character in ID (_)",
            },
            TestCase {
                url: "https://rickastley.co.uk/index.php/tour-dates/",
                description: "completely wrong URL",
            },
            TestCase {
                url: "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8/",
                description: "trailing slash",
            },
            TestCase {
                url: "https://open.spotify.com/album/4PTG3Z6ehGkBFwjybzWkR8",
                description: "album instead of track",
            },
            TestCase {
                url: "https://open.spotify.com/playlist/4PTG3Z6ehGkBFwjybzWkR8",
                description: "playlist instead of track",
            },
            TestCase {
                url: "https://open.spotify.com/",
                description: "empty path",
            },
            TestCase {
                url: "https://open.spotify.com/track/",
                description: "track path without ID",
            },
            TestCase {
                url: "https://open.spotify.com/track/abc-123",
                description: "special characters in ID",
            },
        ];

        for tc in test_cases {
            let url = url::Url::parse(tc.url).unwrap();
            let id = extract_id(&url);
            assert_eq!(id, None, "should fail for: {}", tc.description);
        }
    }
}