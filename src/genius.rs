use std::collections::HashSet;

use anyhow::anyhow;
use genius_rs::search::Hit;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use rspotify::model::FullTrack;
use rustrict::Type;
use scraper::{Html, Selector};
use teloxide::utils::markdown::escape;

use crate::state;

lazy_static! {
    static ref LYRICS_SELECTOR: Selector =
        Selector::parse(".lyrics, [class*=Lyrics__Container], [class*=LyricsPlaceholder__Message]")
            .expect("Should be valid");
}

pub async fn get_lyrics(url: &str) -> anyhow::Result<Vec<String>> {
    let res = Client::new().get(url).send().await?.text().await?;
    let document = Html::parse_document(&res);

    let mut lyrics = vec![];
    document.select(&LYRICS_SELECTOR).for_each(|elem| {
        elem.text().for_each(|text| {
            lyrics.push(text.to_owned());
        });
    });
    if lyrics.is_empty() {
        return Err(anyhow!("Cannot parse lyrics. For some reason for {}", url));
    }
    Ok(lyrics)
}

fn get_type_name(typ: Type) -> String {
    let (lvl, emoji) = if typ.is(Type::SEVERE) {
        ("severe", '⛔')
    } else if typ.is(Type::MODERATE) {
        ("moderate", '🟠')
    } else if typ.is(Type::MILD) {
        ("mild", '🟡')
    } else {
        ("undefined", '❔')
    };

    let typ = if typ.is(Type::PROFANE) {
        "profane"
    } else if typ.is(Type::OFFENSIVE) {
        "offensive"
    } else if typ.is(Type::SEXUAL) {
        "sexual"
    } else if typ.is(Type::MEAN) {
        "mean"
    } else if typ.is(Type::EVASIVE) {
        "evasive"
    } else {
        "undefined ⚫"
    };

    format!("{} {} {}", lvl, typ, emoji)
}

pub fn find_bad_words(lyrics: Vec<String>) -> Vec<String> {
    lyrics
        .into_iter()
        .map(|line| escape(&line))
        .enumerate()
        .filter_map(|(index, line)| {
            let (censored, typ) = rustrict::Censor::from_str(&line)
                .with_censor_first_character_threshold(Type::ANY)
                .with_censor_threshold(Type::INAPPROPRIATE)
                .censor_and_analyze();

            if censored == line {
                return None;
            }

            let highlighted = line
                .chars()
                .into_iter()
                .enumerate()
                .map(|(i, c)| {
                    if !censored.chars().nth(i).contains(&c) {
                        format!("__{}__", c)
                    } else {
                        c.into()
                    }
                })
                .collect::<Vec<_>>()
                .join("")
                .replace("____", "");

            Some(format!(
                "`{}:` {}, `{}`",
                index + 1,
                highlighted,
                get_type_name(typ)
            ))
        })
        .collect()
}

lazy_static! {
    // https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L50
    static ref RG_EXTRA_1: Regex = Regex::new(r"\s-\s.*").expect("Should be compilable");
    static ref RG_EXTRA_2: Regex = Regex::new(r"[^\pL_]+").expect("Should be compilable");
    // https://github.com/khanhas/spicetify-cli/blob/master/CustomApps/lyrics-plus/Utils.js#L41
    static ref RG_FEAT_1: Regex =
        Regex::new(r"(?i)-\s+(feat|with).*").expect("Should be compilable");
    static ref RG_FEAT_2: Regex =
        Regex::new(r"(?i)(\(|\[)(feat|with)\.?\s+.*(\)|\])$").expect("Should be compilable");
}

fn remove_extra_info(name: &str) -> String {
    name.replace(&*RG_EXTRA_1, "")
        .replace(&*RG_EXTRA_2, " ")
        .trim()
        .to_owned()
}

fn remove_song_feat(name: &str) -> String {
    name.replace(&*RG_FEAT_1, "")
        .replace(&*RG_FEAT_2, "")
        .trim()
        .to_owned()
}

fn get_track_names(name: String) -> HashSet<String> {
    let no_extra = remove_extra_info(&name);
    let names = vec![
        name.clone(),
        no_extra.clone(),
        remove_song_feat(&name),
        remove_song_feat(&no_extra),
    ];

    HashSet::from_iter(names.into_iter())
}

pub async fn search_for_track(
    state: &state::UserState,
    track: &FullTrack,
) -> anyhow::Result<Option<Hit>> {
    let artist = track
        .artists
        .iter()
        .map(|art| -> &str { art.name.as_ref() })
        .next()
        .ok_or_else(|| anyhow!("Should be at least 1 artist in track"))?;

    let names = get_track_names(track.name.clone());

    let mut hits_count = 0;

    for (name_i, name) in names.into_iter().enumerate() {
        let q = format!("{} {}", name, artist);

        let hits = state.app.genius.search(q.as_ref()).await?;

        hits_count += hits.len();
        for (hit_i, hit) in hits.into_iter().enumerate() {
            let hit_artist = hit.result.primary_artist.name.as_str();

            if hit_artist.to_lowercase().contains(&artist.to_lowercase())
                || artist.to_lowercase().contains(&hit_artist.to_lowercase())
            {
                log::debug!(
                    "Found text at {} hit with {} name variant ({} - {})",
                    hit_i + 1,
                    name_i + 1,
                    artist,
                    name,
                );
                return Ok(Some(hit));
            }
        }
    }

    log::info!(
        "Found no text in {} hits ({} - {})",
        hits_count,
        artist,
        track.name,
    );

    Ok(None)
}
