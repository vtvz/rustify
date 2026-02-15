use std::sync::LazyLock;
use std::time::Duration;

use backon::{ExponentialBuilder, Retryable as _};
use indoc::formatdoc;
use isolang::Language;
use itertools::Itertools as _;
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
    pub synced_lyrics: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SearchResult {
    confidence: SearchResultConfidence,
    lyrics: Vec<String>,
    indexes: Vec<String>,
    language: Language,
    artist_name: String,
    track_name: String,
}

impl super::SearchResult for SearchResult {
    fn provider(&self) -> super::Provider {
        super::Provider::LrcLib
    }

    fn lyrics(&self) -> Vec<&str> {
        self.lyrics.iter().map(String::as_str).collect()
    }

    fn line_index_name(&self, index: usize) -> String {
        self.indexes
            .get(index)
            .cloned()
            .unwrap_or(index.to_string())
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
            r"
                {text} (with {confidence}% confidence)
                {artist_name} - {track_name}",
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
                .timeout(Duration::from_secs(15))
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
        let artist_name = track.first_artist_name();

        let cmp_artist_name = artist_name.to_lowercase();

        let track_name = track.name();
        let cmp_track_name = track_name.to_lowercase();
        let album_name = &track.album_name();

        let names = get_track_names(track.name());
        let names_len = names.len();

        let mut hits_count = 0;

        for (name_i, name) in names.into_iter().enumerate() {
            let mut url = url::Url::parse("https://lrclib.net/api/search")?;
            url.query_pairs_mut().extend_pairs(&[
                ("artist_name", artist_name),
                ("track_name", track_name),
                ("album_name", album_name),
            ]);

            let res = self.make_request_with_retry(url).await?;

            let hits: Vec<Lyrics> = serde_json::from_str(&res)?;

            hits_count += hits.len();

            for (hit_i, hit) in hits.into_iter().enumerate() {
                let index_lyrics = match (&hit.synced_lyrics, &hit.plain_lyrics) {
                    (Some(lyrics), _) => {
                        static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
                            regex::Regex::new(r"^\[(.*?)\.\d{2}\](.*)$")
                                .expect("Valid regex pattern")
                        });
                        lyrics
                            .lines()
                            .enumerate()
                            .map(|(index, line)| {
                                RE.captures(line).map_or_else(
                                    || (index.to_string(), line.to_owned()),
                                    |caps| (caps[1].to_string(), caps[2].trim().to_owned()),
                                )
                            })
                            .collect()
                    },
                    (_, Some(lyrics)) => lyrics
                        .lines()
                        .enumerate()
                        .map(|(index, line)| (index.to_string(), line.to_owned()))
                        .collect_vec(),
                    _ => continue,
                };

                let (indexes, lyrics): (Vec<_>, Vec<_>) = index_lyrics.into_iter().unzip();

                let confidence = SearchResultConfidence::new(
                    normalized_damerau_levenshtein(
                        &cmp_artist_name,
                        &hit.artist_name.to_lowercase(),
                    ),
                    normalized_damerau_levenshtein(&cmp_track_name, &hit.track_name.to_lowercase()),
                );

                if confidence.confident(BEST_FIT_THRESHOLD) {
                    tracing::trace!(
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
                        language: whatlang::detect_lang(&lyrics.join("\n"))
                            .and_then(|lang| Language::from_639_3(lang.code()))
                            .unwrap_or_default(),
                        indexes,
                        lyrics,
                        artist_name: artist_name.into(),
                        track_name: track_name.into(),
                    }));
                }
            }
        }

        tracing::trace!(
            "Found no text in {} hits in {} name variants ({} - {})",
            hits_count,
            names_len,
            artist_name,
            track_name,
        );

        Ok(None)
    }

    async fn make_request_with_retry(&self, url: url::Url) -> anyhow::Result<String> {
        let send_fn = || async {
            let res = self
                .reqwest
                .get(url.clone())
                .header("Lrclib-Client", "Rustify (https://github.com/vtvz/rustify)")
                .send()
                .await?
                .text()
                .await?;

            anyhow::Ok(res)
        };

        let res = send_fn.retry(ExponentialBuilder::default()).await?;

        Ok(res)
    }
}
