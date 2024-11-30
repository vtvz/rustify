use std::time::Duration;

use cached::proc_macro::io_cached;
use isolang::Language;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use strsim::normalized_damerau_levenshtein;

use super::BEST_FIT_THRESHOLD;
use super::utils::SearchResultConfidence;
use crate::spotify::ShortTrack;

pub struct AZLyrics {
    reqwest: Client,
    service_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AZLyricsHit {
    pub title: String,
    pub link: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SearchResult {
    confidence: SearchResultConfidence,
    lyrics: Vec<String>,
    language: Language,
    title: String,
    url: String,
}

impl super::SearchResult for SearchResult {
    fn provider(&self) -> super::Provider {
        super::Provider::AZLyrics
    }

    fn lyrics(&self) -> Vec<&str> {
        self.lyrics.iter().map(String::as_str).collect()
    }

    fn link(&self) -> String {
        self.url.clone()
    }

    fn link_text(&self, full: bool) -> String {
        let text = if full {
            "AZLyrics"
        } else {
            "Text truncated. Full lyrics can be searched at AZLyrics"
        };

        format!(
            r#"{text} (with {confidence}% confidence)"#,
            confidence = self.confidence,
        )
    }

    fn language(&self) -> Language {
        self.language
    }
}

impl AZLyrics {
    pub fn new(service_url: String) -> anyhow::Result<Self> {
        Ok(Self {
            service_url,
            reqwest: ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
        })
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = track.track_id(),
            track_name = track.track_full_name(),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<Box<dyn super::SearchResult + Send + Sync>>> {
        #[io_cached(
            map_error = r##"|e| anyhow::Error::from(e) "##,
            convert = r#"{ track.track_id().into() }"#,
            ty = "cached::AsyncRedisCache<String, Option<SearchResult>>",
            create = r##" {
                let prefix = module_path!().split("::").last().expect("Will be");
                super::LyricsCacheManager::redis_cache_build(prefix).await.expect("Redis cache should build")
            } "##
        )]
        async fn search_for_track_middleware(
            azlyrics: &AZLyrics,
            track: &ShortTrack,
        ) -> anyhow::Result<Option<SearchResult>> {
            AZLyrics::search_for_track_internal(azlyrics, track).await
        }

        // this weird construction required to make `cached` work
        search_for_track_middleware(self, track)
            .await
            .map(|res| res.map(|opt| Box::new(opt) as _))
    }

    async fn search_for_track_internal(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<SearchResult>> {
        let artist_name = track.first_artist_name();

        let cmp_artist_name = artist_name.to_lowercase();

        let track_name = track.track_name();
        let cmp_track_name = track_name.to_lowercase();

        let q = format!("{} {}", artist_name, track_name);

        let res = self
            .reqwest
            .get(format!("{}/search", self.service_url))
            .query(&[("q", q)])
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let hits: Vec<AZLyricsHit> = serde_json::from_str(&res)?;

        let hits_count = hits.len();

        for (hit_i, hit) in hits.into_iter().enumerate() {
            let confidence = normalized_damerau_levenshtein(
                &format!(r#""{cmp_artist_name}" - {cmp_track_name} lyrics"#),
                hit.title
                    .to_lowercase()
                    .replace("|", "")
                    .replace("-", "")
                    .replace("azlyrics.com", "")
                    .replace("azlyrics", "")
                    .trim(),
            );
            let confidence = SearchResultConfidence::new(confidence, confidence);

            if confidence.confident(BEST_FIT_THRESHOLD) {
                tracing::debug!(
                    confidence = %confidence,
                    "Found text at {} hit of {} ({} - {})",
                    hit_i + 1,
                    hits_count,
                    artist_name,
                    track_name,
                );

                let res = self
                    .reqwest
                    .get(format!("{}/lyrics", self.service_url))
                    .query(&[("url", &hit.link)])
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await?;

                #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
                #[serde(rename_all = "camelCase")]
                pub struct Lyrics {
                    pub lyrics: String,
                }
                let Lyrics { lyrics } = serde_json::from_str(&res)?;

                return Ok(Some(SearchResult {
                    confidence,
                    language: whatlang::detect_lang(&lyrics)
                        .and_then(|lang| Language::from_639_3(lang.code()))
                        .unwrap_or_default(),
                    lyrics: lyrics.lines().map(str::to_string).collect(),
                    title: hit.title,
                    url: hit.link,
                }));
            }
        }

        tracing::info!(
            "Found no text in {} hits ({} - {})",
            hits_count,
            artist_name,
            track_name,
        );

        Ok(None)
    }
}
