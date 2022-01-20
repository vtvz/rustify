use anyhow::anyhow;
use indoc::formatdoc;
use rspotify::clients::BaseClient;
use rspotify::model::Modality;
use teloxide::prelude::*;
use teloxide::utils::markdown::escape;

use crate::state::UserState;
use crate::{genius, spotify, CurrentlyPlaying, ParseMode};

pub async fn handle(cx: &UpdateWithCx<Bot, Message>, state: &UserState) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;
    let track = match spotify::currently_playing(&*spotify).await {
        CurrentlyPlaying::Err(err) => return Err(err),
        CurrentlyPlaying::None(message) => {
            cx.answer(message).send().await?;

            return Ok(true);
        }
        CurrentlyPlaying::Ok(track) => track,
    };

    let track_id = track.id.clone().expect("Should be prevalidated");
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
            🎶 `{}` {} ⌛ {} BPM
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
        cx.answer(format!("{}\n\n{}\n\n`No lyrics found`", spotify::create_track_name(&track), features.trim()))
            .parse_mode(ParseMode::MarkdownV2)
            .send()
            .await?;

        return Ok(true);
    };

    let lyrics = genius::get_lyrics(&hit.result.url).await?;

    let mut lines = lyrics.len();
    let message = loop {
        if lines == 0 {
            return Err(anyhow!("Issues with lyrics"));
        }

        let message = format!(
            "{}\n\n{}\n\n{}\n\n[{}]({})",
            spotify::create_track_name(&track),
            features.trim(),
            escape(&lyrics[0..lines].join("\n")),
            if lines == lyrics.len() {
                "Genius Source"
            } else {
                "Text truncated. Full lyrics can be found at Genius"
            },
            hit.result.url
        );

        if message.len() <= 4096 {
            break message;
        }

        lines -= 1;
    };

    cx.answer(message)
        .parse_mode(ParseMode::MarkdownV2)
        .send()
        .await?;

    Ok(true)
}
