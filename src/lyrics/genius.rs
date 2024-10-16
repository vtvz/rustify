#![allow(dead_code)]

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use anyhow::{anyhow, Context};
use cached::proc_macro::cached;
use indoc::formatdoc;
use isolang::Language;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{Client, StatusCode};
use rspotify::model::FullTrack;
use rustrict::is_whitespace;
use strsim::normalized_damerau_levenshtein;
use teloxide::utils::html;

use crate::spotify;

pub struct GeniusLocal {
    token: String,
    service_url: String,
    reqwest: Client,
}

#[derive(Serialize, Deserialize)]
pub struct LyricsResponse {
    lyrics: String,
}

impl GeniusLocal {
    pub fn new(service_url: String, token: String) -> Self {
        Self {
            token,
            service_url,
            reqwest: Client::new(),
        }
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = %spotify::utils::get_track_id(track),
            track_name = %spotify::utils::create_track_name(track),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &FullTrack,
    ) -> anyhow::Result<Option<SearchResult>> {
        let res = search_for_track(self, track).await?;

        let Some(mut res) = res else { return Ok(None) };

        let lyrics = self.get_lyrics(&res).await?;

        if lyrics.is_empty() {
            return Ok(None);
        }

        res.lyrics = lyrics;
        res.language = detect_language(track, &res.lyrics);

        Ok(Some(res))
    }

    async fn get_lyrics(&self, hit: &SearchResult) -> anyhow::Result<Vec<String>> {
        let res = self
            .reqwest
            .get(format!("{}/{}/lyrics", self.service_url, hit.id))
            .header("Authorization", &self.token)
            .send()
            .await?;

        if res.status() == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }

        let res: LyricsResponse = res.error_for_status()?.json().await?;

        if res.lyrics.is_empty() {
            return Err(anyhow!("Cannot get lyrics, for some reason for {}", hit.id));
        }

        Ok(res.lyrics.lines().map(String::from).collect())
    }
}

#[derive(Clone)]
pub struct SearchResult {
    id: u32,
    url: String,
    title: String,
    confidence: SearchResultConfidence,
    lyrics: Vec<String>,
    language: Language,
}

impl super::SearchResult for SearchResult {
    fn provider(&self) -> super::Provider {
        super::Provider::Genius
    }

    fn lyrics(&self) -> Vec<&str> {
        self.lyrics.iter().map(String::as_str).collect()
    }

    fn language(&self) -> Language {
        self.language
    }

    fn tg_link(&self, full: bool) -> String {
        let text = if full {
            "Genius Source"
        } else {
            "Text truncated. Full lyrics can be found at Genius"
        };

        formatdoc!(
            r#"
                <a href="{url}">{text} (with {confidence}% confidence)
                {title}</a>
            "#,
            text = html::escape(text),
            title = html::escape(&self.title),
            confidence = self.confidence,
            url = self.url
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SearchResultConfidence {
    title: f64,
    artist: f64,
}

impl SearchResultConfidence {
    fn new(artist: f64, title: f64) -> Self {
        Self { title, artist }
    }

    fn confident(&self, threshold: f64) -> bool {
        self.artist >= threshold && self.title >= threshold
    }

    fn avg(&self) -> f64 {
        (self.title + self.artist) / 2.0
    }
}

impl Display for SearchResultConfidence {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}", self.avg() * 100.0)
    }
}

#[cached(
    key = "String",
    convert = r#"{ format!("{:?}", _track.id) }"#,
    sync_writes = true,
    size = 100,
    time = 3600,
    time_refresh = true
)]
pub fn detect_language(_track: &FullTrack, lyrics: &[String]) -> Language {
    whatlang::detect_lang(&lyrics.join("\n"))
        .and_then(|lang| Language::from_639_3(lang.code()))
        .unwrap_or_default()
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeniusArtist {
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeniusHit {
    id: u32,
    url: String,
    full_title: String,
    title: String,
    #[serde(rename = "artist")]
    primary_artist: GeniusArtist,
}

/// Returns url to Genius page with some additional information
#[cached(
    key = "String",
    convert = r#"{ format!("{:?}", track.id) }"#,
    result = true,
    sync_writes = true,
    size = 100,
    time = 3600,
    time_refresh = true
)]
#[tracing::instrument(
    name = "search_for_track_internal"
    skip_all,
    fields(
        track_id = %spotify::utils::get_track_id(track),
        track_name = %spotify::utils::create_track_name(track),
    )
)]
async fn search_for_track(
    genius: &GeniusLocal,
    track: &FullTrack,
) -> anyhow::Result<Option<SearchResult>> {
    const THRESHOLD: f64 = 0.45;

    let artist = track
        .artists
        .iter()
        .map(|art| -> &str { art.name.as_ref() })
        .next()
        .context("Should be at least 1 artist in track")?;

    let names = get_track_names(&track.name);
    let names_len = names.len();

    let cmp_artist = artist.to_lowercase();
    let cmp_title = track.name.to_lowercase();

    let mut hits_count = 0;

    for (name_i, name) in names.into_iter().enumerate() {
        let q = format!("{} {}", name, artist);

        let hits: Vec<GeniusHit> = genius
            .reqwest
            .get(format!("{}/search", genius.service_url))
            .header("Authorization", &genius.token)
            .query(&[("q", q)])
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        hits_count += hits.len();
        for (hit_i, hit) in hits.into_iter().enumerate() {
            let title = hit
                .full_title
                .replace(is_whitespace, " ")
                .trim()
                .to_string();

            let confidence = SearchResultConfidence::new(
                normalized_damerau_levenshtein(
                    &cmp_artist,
                    hit.primary_artist.name.to_lowercase().as_str(),
                ),
                normalized_damerau_levenshtein(&cmp_title, hit.title.to_lowercase().as_str()),
            );

            if confidence.confident(THRESHOLD) {
                tracing::debug!(
                    confidence = %confidence,
                    "Found text at {} hit with {} name variant ({} - {}) with name '{}'",
                    hit_i + 1,
                    name_i + 1,
                    artist,
                    name,
                    title,
                );

                return Ok(Some(SearchResult {
                    id: hit.id,
                    url: hit.url,
                    title,
                    confidence,
                    lyrics: Default::default(),
                    language: Default::default(),
                }));
            }
        }
    }

    tracing::info!(
        "Found no text in {} hits in {} name variants ({} - {})",
        hits_count,
        names_len,
        artist,
        track.name,
    );

    Ok(None)
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

fn get_track_names(name: &str) -> HashSet<String> {
    let no_extra = remove_extra_info(name);
    let names = vec![
        name.to_owned(),
        no_extra.clone(),
        remove_song_feat(name),
        remove_song_feat(&no_extra),
    ];

    HashSet::from_iter(names)
}
