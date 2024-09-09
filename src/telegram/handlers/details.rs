use std::collections::HashSet;

use anyhow::{anyhow, Context};
use convert_case::{Case, Casing};
use indoc::formatdoc;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::clients::BaseClient;
use rspotify::model::{FullTrack, Id, Modality, TrackId};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup, ReplyParameters};
use teloxide::utils::markdown;

use crate::entity::prelude::*;
use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::TrackStatusService;
use crate::{profanity, spotify, telegram};

pub async fn handle_current(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;
    let track = match CurrentlyPlaying::get(&spotify).await {
        CurrentlyPlaying::Err(err) => return Err(err.into()),
        CurrentlyPlaying::None(message) => {
            bot.send_message(m.chat.id, message.to_string())
                .send()
                .await?;

            return Ok(true);
        },
        CurrentlyPlaying::Ok(track, _) => track,
    };

    common(m, bot, state, *track).await
}

fn extract_id(url: &str) -> Option<TrackId> {
    lazy_static! {
        static ref RE: Regex = Regex::new("^/track/([a-zA-Z0-9]+)$").expect("Should be compilable");
    }

    let url = url::Url::parse(url).ok()?;

    let cap = RE.captures(url.path())?;

    let id = TrackId::from_id(cap[1].to_owned());

    id.ok()
}

pub async fn handle_url(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let Some(text) = m.text() else {
        return Ok(false);
    };

    let Some(track_id) = extract_id(text) else {
        return Ok(false);
    };

    let track = state.spotify.read().await.track(track_id, None).await?;

    common(m, bot, state, track).await
}

async fn common(
    m: &Message,
    bot: &Bot,
    state: &UserState,
    track: FullTrack,
) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;

    let track_id = track.id.clone().context("Should be prevalidated")?;

    let status =
        TrackStatusService::get_status(state.app.db(), &state.user_id, track_id.id()).await;

    let keyboard = match status {
        TrackStatus::Disliked => {
            vec![vec![InlineButtons::Cancel(track_id.id().to_owned()).into()]]
        },
        TrackStatus::Ignore | TrackStatus::None => {
            vec![vec![
                InlineButtons::Dislike(track_id.id().to_owned()).into(),
            ]]
        },
    };

    let features = spotify.track_features(track_id.clone()).await?;

    let modality = match features.mode {
        Modality::Minor => "Minor",
        Modality::Major => "Major",
        Modality::NoResult => "Something",
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

    let disliked_by = TrackStatusService::count_status(
        state.app.db(),
        TrackStatus::Disliked,
        None,
        Some(track_id.id()),
    )
    .await?;

    let ignored_by = TrackStatusService::count_status(
        state.app.db(),
        TrackStatus::Ignore,
        None,
        Some(track_id.id()),
    )
    .await?;

    let genres: HashSet<_> = {
        let artist_ids: Vec<_> = track
            .artists
            .iter()
            .filter_map(|artist| artist.id.clone())
            .collect();

        let artists = match spotify.artists(artist_ids).await {
            // HACK: 403 "Spotify is unavailable in this country" error
            Err(rspotify::ClientError::Http(box rspotify::http::HttpError::StatusCode(resp))) => {
                tracing::info!("Resp from artists fetching {:?}", resp.text().await);

                vec![]
            },
            // HACK: https://github.com/ramsayleung/rspotify/issues/452
            Err(rspotify::ClientError::ParseJson(err)) => {
                tracing::info!(
                    "Spotify changes API to produce floats instead of ints. Ignore for now {:?}",
                    err
                );

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
            .map(|(genre, url)| format!("[{genre}]({url})", genre = markdown::escape(&genre)))
            .collect()
    };

    let genres_line = if genres.is_empty() {
        "".into()
    } else {
        format!("🎭 Genres: {}\n", genres.iter().join(", "))
    };

    let features: String = formatdoc! {
        "
            🎶 `{} {}` ⌛ {:.0} BPM
            🎻 Acoustic {:.0}%
            🕺 Suitable for dancing {:.0}%
            ⚡️ Energetic {:.0}%
            🤐 Without vocal {:.0}%
            🏟 Performed live {:.0}%
            🎤 Speech\\-like {:.0}%
            ☺️ Positiveness {:.0}%
            👎 Disliked by {} people
            🙈 Ignored by {} people
        ",
        key,
        modality,
        features.tempo,
        features.acousticness * 100.0,
        features.danceability * 100.0,
        features.energy * 100.0,
        features.instrumentalness * 100.0,
        features.liveness * 100.0,
        features.speechiness * 100.0,
        features.valence * 100.0,
        disliked_by,
        ignored_by,
    };

    let Some(hit) = state.app.lyrics().search_for_track(&track).await? else {
        bot.send_message(
            m.chat.id,
            formatdoc!(
                "
                    {track_name}

                    {features}
                    {genres_line}
                    `No lyrics found`
                ",
                track_name = spotify::utils::create_track_tg_link(&track),
                features = features.trim(),
                genres_line = genres_line,
            ),
        )
        .reply_parameters(ReplyParameters::new(m.id))
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .send()
        .await?;

        return Ok(true);
    };

    let checked = profanity::Manager::check(hit.lyrics());

    let lyrics: Vec<_> = checked.iter().map(|line| line.highlighted()).collect();

    let typ = checked.typ.to_string();

    let mut lines = lyrics.len();
    let message = loop {
        if lines == 0 {
            return Err(anyhow!("Issues with lyrics"));
        }

        let message = formatdoc!(
            "
                {track_name}

                {features}
                🤬 Profanity `{profanity}`
                🌐 Language: {language}
                {genres_line}
                {lyrics}

                {genius}
            ",
            track_name = spotify::utils::create_track_tg_link(&track),
            features = features.trim(),
            profanity = typ,
            lyrics = &lyrics[0..lines].join("\n"),
            language = hit.language(),
            genres_line = genres_line,
            genius = hit.tg_link(lines == lyrics.len())
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    bot.send_message(m.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_parameters(ReplyParameters::new(m.id))
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .send()
        .await?;

    Ok(true)
}
