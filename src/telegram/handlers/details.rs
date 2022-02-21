use std::str::FromStr;

use anyhow::anyhow;
use indoc::formatdoc;
use lazy_static::lazy_static;
use regex::Regex;
use rspotify::clients::BaseClient;
use rspotify::model::{FullTrack, Id, Modality, TrackId};
use teloxide::prelude2::*;
use teloxide::types::{InlineKeyboardMarkup, ParseMode, ReplyMarkup};

use crate::spotify::CurrentlyPlaying;
use crate::state::UserState;
use crate::telegram::inline_buttons::InlineButtons;
use crate::track_status_service::{Status, TrackStatusService};
use crate::{genius, profanity, spotify, telegram};

pub async fn handle_current(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;
    let track = match spotify::currently_playing(&*spotify).await {
        CurrentlyPlaying::Err(err) => return Err(err),
        CurrentlyPlaying::None(message) => {
            bot.send_message(m.chat.id, message).send().await?;

            return Ok(true);
        }
        CurrentlyPlaying::Ok(track) => track,
    };

    return common(m, bot, state, *track).await;
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
pub async fn handle_url(m: &Message, bot: &Bot, state: &UserState) -> anyhow::Result<bool> {
    let Some(text) = m.text() else {
        return Ok(false);
    };

    let Some(track_id) = extract_id(text) else {
        return Ok(false);
    };

    let track = state.spotify.read().await.track(&track_id).await?;

    return common(m, bot, state, track).await;
}

async fn common(
    m: &Message,
    bot: &Bot,
    state: &UserState,
    track: FullTrack,
) -> anyhow::Result<bool> {
    let spotify = state.spotify.read().await;

    let track_id = track.id.clone().expect("Should be prevalidated");

    let status = TrackStatusService::get_status(&state.app.db, &state.user_id, track_id.id()).await;

    let keyboard = match status {
        Status::Disliked => {
            vec![vec![InlineButtons::Cancel(track_id.id().to_owned()).into()]]
        }
        Status::Ignore | Status::None => {
            vec![vec![InlineButtons::Dislike(track_id.id().to_owned()).into()]]
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

    let disliked_by =
        TrackStatusService::count_track_status(&state.app.db, track_id.id(), Status::Disliked)
            .await?;

    let ignored_by =
        TrackStatusService::count_track_status(&state.app.db, track_id.id(), Status::Ignore)
            .await?;

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

    let Some(hit) = genius::search_for_track(state, &track).await? else {
        bot.send_message(m.chat.id,
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

    let lyrics = genius::get_lyrics(&hit.url).await?;

    let checked = profanity::Manager::check(lyrics);

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
                
                {lyrics}
                
                {genius}
            ",
            track_name = spotify::create_track_name(&track),
            features = features.trim(),
            profanity = typ,
            lyrics = &lyrics[0..lines].join("\n"),
            genius = hit.tg_link(if lines == lyrics.len() {
                "Genius Source"
            } else {
                "Text truncated. Full lyrics can be found at Genius"
            })
        );

        if message.len() <= telegram::MESSAGE_MAX_LEN {
            break message;
        }

        lines -= 1;
    };

    bot.send_message(m.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .reply_markup(ReplyMarkup::InlineKeyboard(InlineKeyboardMarkup::new(
            keyboard,
        )))
        .send()
        .await?;

    Ok(true)
}
