use std::collections::HashSet;

use anyhow::anyhow;
use convert_case::{Case, Casing};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::clients::BaseClient;
use rspotify::model::{Modality, TrackId};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode};

use crate::app::App;
use crate::entity::prelude::*;
use crate::spotify::{CurrentlyPlaying, ShortTrack};
use crate::telegram::handlers::HandleStatus;
use crate::telegram::inline_buttons::InlineButtons;
use crate::telegram::utils::link_preview_small_top;
use crate::track_status_service::TrackStatusService;
use crate::user::UserState;
use crate::{profanity, telegram};

pub async fn handle_current(
    app: &'static App,
    state: &UserState,
    chat_id: &ChatId,
) -> anyhow::Result<HandleStatus> {
    let spotify = state.spotify().await;
    let track = match CurrentlyPlaying::get(&spotify).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            app.bot()
                .send_message(*chat_id, message.localize(state.locale()))
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

pub async fn handle_url(
    app: &'static App,
    state: &UserState,
    url: &url::Url,
    m: &Message,
) -> anyhow::Result<HandleStatus> {
    let Some(track_id) = extract_id(url) else {
        return Ok(HandleStatus::Skipped);
    };

    let track = state.spotify().await.track(track_id, None).await?.into();

    common(app, state, &m.chat.id, track).await
}

async fn common(
    app: &'static App,
    state: &UserState,
    chat_id: &ChatId,
    track: ShortTrack,
) -> anyhow::Result<HandleStatus> {
    let m = app
        .bot()
        .send_message(
            *chat_id,
            t!("details.collecting-info", locale = state.locale()),
        )
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
                    .push(&format!(r#"genre:"{}""#, genre));

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
        format!("🎭 Genres: {}\n", genres.iter().join(", "))
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
            .edit_message_text(
                *chat_id,
                m.id,
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

    let checked = profanity::Manager::check(hit.lyrics());

    let lyrics: Vec<_> = checked.iter().map(|line| line.highlighted()).collect();

    let typ = checked.typ.to_string();

    let mut lines = lyrics.len();
    // This requires to fit lyrics to tg message
    let message = loop {
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

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    if app.analyze().is_some() {
        keyboard.push(vec![
            InlineButtons::Analyze(track.id().to_owned())
                .into_inline_keyboard_button(state.locale()),
        ]);
    }

    app.bot()
        .edit_message_text(*chat_id, m.id, message)
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
        let never_gonna_give_you_up =
            "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8?si=b248017abca04ef0";

        let url = url::Url::parse(never_gonna_give_you_up).unwrap();
        let id = extract_id(&url);

        assert_eq!(
            id,
            Some(TrackId::from_id("4PTG3Z6ehGkBFwjybzWkR8").unwrap())
        );
    }

    #[test]
    fn extract_id_broken() {
        let gonna_give_you_up =
            "https://open.spotify.com/track/4PTG3Z6ehGkBFwjybzWkR8_?si=b248017abca04ef0";

        let url = url::Url::parse(gonna_give_you_up).unwrap();
        let id = extract_id(&url);

        assert_eq!(id, None);
    }

    #[test]
    fn extract_id_wrong() {
        let gonna_give_you_up = "https://rickastley.co.uk/index.php/tour-dates/";

        let url = url::Url::parse(gonna_give_you_up).unwrap();
        let id = extract_id(&url);

        assert_eq!(id, None);
    }
}
