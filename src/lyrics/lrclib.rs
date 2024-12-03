use std::time::Duration;

use cached::proc_macro::io_cached;
use indoc::formatdoc;
use isolang::Language;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use strsim::normalized_damerau_levenshtein;

use super::BEST_FIT_THRESHOLD;
use super::utils::get_track_names;
use crate::lyrics::utils::SearchResultConfidence;
use crate::spotify::ShortTrack;

pub struct LrcLib {
    reqwest: Client,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub id: i64,
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SearchResult {
    confidence: SearchResultConfidence,
    lyrics: Vec<String>,
    language: Language,
    artist_name: String,
    track_name: String,
}

impl super::SearchResult for SearchResult {
    fn provider(&self) -> super::Provider {
        super::Provider::LrcLib
    }

    fn lyrics(&self) -> Vec<&str> {
        self.lyrics.iter().map(|lyrics| lyrics.as_str()).collect()
    }

    fn link(&self) -> String {
        let url = url::Url::parse("https://lrclib.net/search/").expect("If it fails, it fails");

        url.join(&format!("{} {}", self.artist_name, self.track_name))
            .map(|url| url.to_string())
            .unwrap_or("https://lrclib.net/".into())
    }

    fn link_text(&self, full: bool) -> String {
        let text = if full {
            "LrcLib"
        } else {
            "Text truncated. Full lyrics can be searched at LrcLib"
        };

        formatdoc!(
            r#"
                {text} (with {confidence}% confidence)
                {artist_name} - {track_name}
            "#,
            confidence = self.confidence,
            artist_name = self.artist_name,
            track_name = self.track_name,
        )
    }

    fn language(&self) -> Language {
        self.language
    }
}

impl LrcLib {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            reqwest: ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
        })
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = track.id(),
            track_name = track.name_with_artists(),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<Box<dyn super::SearchResult + Send + Sync>>> {
        #[io_cached(
            map_error = r##"|e| anyhow::Error::from(e) "##,
            convert = r#"{ track.id().into() }"#,
            ty = "cached::AsyncRedisCache<String, Option<SearchResult>>",
            create = r##" {
                let prefix = module_path!().split("::").last().expect("Will be");
                super::LyricsCacheManager::redis_cached_build(prefix).await.expect("Redis cache should build")
            } "##
        )]
        async fn search_for_track_middleware(
            lrclib: &LrcLib,
            track: &ShortTrack,
        ) -> anyhow::Result<Option<SearchResult>> {
            LrcLib::search_for_track_internal(lrclib, track).await
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

        let track_name = track.name();
        let cmp_track_name = track_name.to_lowercase();
        let album_name = &track.album_name();

        let names = get_track_names(track.name());
        let names_len = names.len();

        let mut hits_count = 0;

        for (name_i, name) in names.into_iter().enumerate() {
            let mut url = reqwest::Url::parse("https://lrclib.net/api/search")?;
            url.query_pairs_mut().extend_pairs(&[
                ("artist_name", artist_name),
                ("track_name", track_name),
                ("album_name", album_name),
            ]);

            let res = self
                .reqwest
                .get(url)
                .header("Lrclib-Client", "Rustify (https://github.com/vtvz/rustify)")
                .send()
                .await?
                .text()
                .await?;

            let hits: Vec<Lyrics> = serde_json::from_str(&res)?;

            hits_count += hits.len();

            for (hit_i, hit) in hits.into_iter().enumerate() {
                let Some(hit_plain_lyrics) = hit.plain_lyrics else {
                    continue;
                };

                let confidence = SearchResultConfidence::new(
                    normalized_damerau_levenshtein(
                        &cmp_artist_name,
                        &hit.artist_name.to_lowercase(),
                    ),
                    normalized_damerau_levenshtein(&cmp_track_name, &hit.track_name.to_lowercase()),
                );

                if confidence.confident(BEST_FIT_THRESHOLD) {
                    tracing::debug!(
                        confidence = %confidence,
                        "Found text at {} hit with {} name variant ({} - {}) with name '{}'",
                        hit_i + 1,
                        name_i + 1,
                        artist_name,
                        name,
                        track_name,
                    );

                    return Ok(Some(SearchResult {
                        confidence,
                        language: whatlang::detect_lang(&hit_plain_lyrics)
                            .and_then(|lang| Language::from_639_3(lang.code()))
                            .unwrap_or_default(),
                        lyrics: hit_plain_lyrics.lines().map(str::to_string).collect(),
                        artist_name: artist_name.into(),
                        track_name: track_name.into(),
                    }));
                }
            }
        }

        tracing::info!(
            "Found no text in {} hits in {} name variants ({} - {})",
            hits_count,
            names_len,
            artist_name,
            track_name,
        );

        Ok(None)
    }
}
