#![allow(dead_code)]

use std::time::Duration;

use anyhow::anyhow;
use indoc::formatdoc;
use isolang::Language;
use reqwest::{Client, ClientBuilder, StatusCode};
use rustrict::is_whitespace;
use strsim::normalized_damerau_levenshtein;

use super::utils::get_track_names;
use crate::lyrics::BEST_FIT_THRESHOLD;
use crate::lyrics::utils::SearchResultConfidence;
use crate::spotify::ShortTrack;

#[derive(Clone, Serialize, Deserialize)]
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

    fn link(&self) -> String {
        self.url.clone()
    }

    fn link_text(&self, full: bool) -> String {
        let text = if full {
            "Genius Source"
        } else {
            "Text truncated. Full lyrics can be found at Genius"
        };

        formatdoc!(
            r"
                {text} (with {confidence}% confidence)
                {title}
            ",
            title = &self.title,
            confidence = self.confidence,
        )
    }
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

pub struct GeniusLocal {
    token: String,
    service_url: String,
    reqwest: Client,
}

impl GeniusLocal {
    pub fn new(service_url: String, token: String) -> anyhow::Result<Self> {
        Ok(Self {
            token,
            service_url,
            reqwest: ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
        })
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = %track.id(),
            track_name = %track.name_with_artists(),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<SearchResult>> {
        let res = self.find_best_fit_entry(track).await?;

        let Some(mut res) = res else { return Ok(None) };

        let lyrics = self.get_lyrics(&res).await?;

        if lyrics.is_empty() {
            return Ok(None);
        }

        res.language = whatlang::detect_lang(&lyrics.join("\n"))
            .and_then(|lang| Language::from_639_3(lang.code()))
            .unwrap_or_default();

        res.lyrics = lyrics;

        Ok(Some(res))
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = %track.id(),
            track_name = %track.name_with_artists(),
        )
    )]
    async fn find_best_fit_entry(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<SearchResult>> {
        let artist = track.first_artist_name();

        let names = get_track_names(track.name());
        let names_len = names.len();

        let cmp_artist = artist.to_lowercase();
        let cmp_title = track.name().to_lowercase();

        let mut hits_count = 0;

        for (name_i, name) in names.into_iter().enumerate() {
            let q = format!("{name} {artist}");

            let hits: Vec<GeniusHit> = self
                .reqwest
                .get(format!("{}/search", self.service_url))
                .header("Authorization", &self.token)
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

                if confidence.confident(BEST_FIT_THRESHOLD) {
                    tracing::trace!(
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
                        lyrics: vec![],
                        language: Language::default(),
                    }));
                }
            }
        }

        tracing::trace!(
            "Found no text in {} hits in {} name variants ({} - {})",
            hits_count,
            names_len,
            artist,
            track.name(),
        );

        Ok(None)
    }

    async fn get_lyrics(&self, hit: &SearchResult) -> anyhow::Result<Vec<String>> {
        #[derive(Serialize, Deserialize)]
        struct LyricsResponse {
            lyrics: String,
        }

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
