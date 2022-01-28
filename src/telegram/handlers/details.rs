use std::str::FromStr;

use anyhow::anyhow;
use indoc::formatdoc;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::clients::BaseClient;
use rspotify::model::{FullTrack, Modality, TrackId};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::{Status, TrackStatusService};
use crate::{genius, profanity, spotify, telegram};

pub async fn handle_current(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState,
) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;
    let track = match spotify::currently_playing(&*spotify).await {
        CurrentlyPlaying::Err(err) => return Err(err),
        CurrentlyPlaying::None(message) => {
            cx.answer(message).send().await?;

            return Ok(true);
        }
        CurrentlyPlaying::Ok(track) => track,
    };

    return common(cx, state, *track).await;
}

fn extract_id(url: &str) -> Option<TrackId> {
    lazy_static! {
        static ref RE: Regex = Regex::new("^/track/([a-zA-Z0-9]+)$").expect("Should be compilable");
    }

    let Ok(url) = url::Url::parse(url) else {
        return None;
    };

    let Some(cap) = RE.captures(url.path()) else {
        return None;
    };

    let id = TrackId::from_str(&cap[1]);

    id.ok()
}
pub async fn handle_url(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState,
) -> anyhow::Result<bool> {
    let Some(text) = cx.update.text() else {
        return Ok(false);
    };

    let Some(track_id) = extract_id(text) else {
        return Ok(false);
    };

    let track = state.spotify.read().await.track(&track_id).await?;

    return common(cx, state, track).await;
}

async fn common(
    cx: &UpdateWithCx<Bot, Message>,
    state: &UserState,
    track: FullTrack,
) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;

    let track_id = track.id.clone().expect("Should be prevalidated");

    let status = TrackStatusService::get_status(
        &state.app.db,
        &state.user_id,
        &spotify::get_track_id(&track),
    )
    .await;

    let keyboard = match status {
        Status::Disliked => {
            vec![vec![
                InlineButtons::Cancel(spotify::get_track_id(&track)).into()
            ]]
        }
        Status::Ignore | Status::None => {
            vec![vec![
                InlineButtons::Dislike(spotify::get_track_id(&track)).into()
            ]]
        }
    };

    let features = spotify.track_features(&track_id).await?;

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

    let round = |num: f32| (num * 100.0).round() as u32;

    let features: String = formatdoc! {
        "
            🎶 `{} {}` ⌛ {} BPM
            🎻 Acoustic {}%
            🕺 Suitable for dancing {}%
            ⚡️ Energetic {}%
            🤐 Without vocal {}%
            🏟 Performed live {}%
            🎤 Speech\\-like {}%
            ☺️ Positiveness {}%
        ",
        key,
        modality,
        features.tempo.round() as u32,
        round(features.acousticness),
        round(features.danceability),
        round(features.energy),
        round(features.instrumentalness),
        round(features.liveness),
        round(features.speechiness),
        round(features.valence),
    };

    let Some(hit) = genius::search_for_track(state, &track).await? else {
        cx
            .answer(
                formatdoc!(
                    "
                        {track_name}
                        
                        {features}
                        
                        `No lyrics found`
                    ",
                    track_name = spotify::create_track_name(&track),
                    features = features.trim(),
                )
            )
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
                keyboard,
            )))
            .send()
            .await?;

        return Ok(true);
    };

    let lyrics = genius::get_lyrics(&hit).await?;

    let checked = profanity::Manager::check(lyrics);

    let lyrics: Vec<_> = checked.iter().map(|line| line.highlighted()).collect();

    let typ = checked.sum_type_name();

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
                
                {lyrics}
                
                [{genius_line}]({genius_link})
            ",
            track_name = spotify::create_track_name(&track),
            features = features.trim(),
            profanity = typ,
            lyrics = &lyrics[0..lines].join("\n"),
            genius_line = if lines == lyrics.len() {
                "Genius Source"
            } else {
                "Text truncated\\. Full lyrics can be found at Genius"
            },
            genius_link = hit
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    cx.answer(message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .send()
        .await?;

    Ok(true)
}
