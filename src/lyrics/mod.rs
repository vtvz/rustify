use std::sync::LazyLock;
use std::time::Duration;

use genius::GeniusLocal;
use isolang::Language;
use lrclib::LrcLib;
use musixmatch::Musixmatch;
use serde::Serialize;
use serde::de::DeserializeOwned;
use strum_macros::Display;
use tokio::sync::RwLock;

use crate::infrastructure::cache::CacheManager;
use crate::spotify::ShortTrack;

pub mod genius;
pub mod lrclib;
pub mod musixmatch;
pub mod utils;

pub const BEST_FIT_THRESHOLD: f64 = 0.6;

#[derive(Display)]
pub enum Provider {
    Musixmatch,
    Genius,
    LrcLib,
}

pub trait SearchResult {
    fn provider(&self) -> Provider;
    fn lyrics(&self) -> Vec<&str>;
    fn link(&self) -> String;
    fn link_text(&self, full: bool) -> String;

    fn language(&self) -> Language;

    fn line_index_name(&self, index: usize) -> String {
        (index + 1).to_string()
    }
}

pub struct Manager {
    genius: GeniusLocal,
    musixmatch: Musixmatch,
    lrclib: LrcLib,
}

impl Manager {
    pub fn new(
        genius_service_url: String,
        genius_token: String,
        musixmatch_tokens: impl IntoIterator<Item = String>,
    ) -> anyhow::Result<Self> {
        let genius = GeniusLocal::new(genius_service_url, genius_token)?;
        let musixmatch = Musixmatch::new(musixmatch_tokens)?;
        let lrclib = LrcLib::new()?;

        Ok(Self {
            genius,
            musixmatch,
            lrclib,
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
    ) -> anyhow::Result<Option<Box<dyn SearchResult + Send>>> {
        // tired to fight type system to handle this with vec
        macro_rules! handle_provider {
            ($name:expr, $provider:expr) => {
                let result = $provider.search_for_track(track).await;

                match result {
                    Ok(Some(res)) => {
                        return Ok(Some(res));
                    },
                    Err(err) => {
                        tracing::error!(
                            err = ?err,
                            "Error with {} occurred",
                            $name
                        );
                    },
                    _ => {
                        tracing::trace!("{} text not found", $name);
                    },
                };
            };
        }

        let priority = [Provider::LrcLib, Provider::Genius, Provider::Musixmatch];

        for provider in priority {
            match provider {
                Provider::Musixmatch => {
                    handle_provider!("Musixmatch", self.musixmatch);
                },
                Provider::Genius => {
                    handle_provider!("Genius", self.genius);
                },
                Provider::LrcLib => {
                    handle_provider!("LrcLib", self.lrclib);
                },
            }
        }

        Ok(None)
    }
}

#[derive(Debug)]
pub struct LyricsCacheManager {}

static LYRICS_CACHE_TTL: LazyLock<RwLock<u64>> = LazyLock::new(|| RwLock::new(24 * 60 * 60));

impl LyricsCacheManager {
    pub async fn init(lyrics_cache_ttl: u64) {
        let mut lock = LYRICS_CACHE_TTL.write().await;
        *lock = lyrics_cache_ttl;
    }

    pub async fn redis_cached_build<T: Sync + Send + Serialize + DeserializeOwned>(
        provider: &str,
    ) -> anyhow::Result<cached::AsyncRedisCache<String, T>> {
        CacheManager::redis_cached_build(
            &format!("lyrics:{provider}"),
            Duration::from_secs(*LYRICS_CACHE_TTL.read().await),
        )
        .await
    }
}
