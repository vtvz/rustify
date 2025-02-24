use azlyrics::AZLyrics;
use genius::GeniusLocal;
use isolang::Language;
use lazy_static::lazy_static;
use lrclib::LrcLib;
use musixmatch::Musixmatch;
use serde::Serialize;
use serde::de::DeserializeOwned;
use strum_macros::Display;
use tokio::sync::RwLock;

use crate::cache::CacheManager;
use crate::spotify::ShortTrack;

pub mod azlyrics;
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
    AZLyrics,
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
    #[allow(dead_code)]
    azlyrics: AZLyrics,
}

impl Manager {
    pub fn new(
        genius_service_url: String,
        genius_token: String,
        musixmatch_tokens: impl IntoIterator<Item = String>,
        azlyrics_service_url: String,
    ) -> anyhow::Result<Self> {
        let genius = GeniusLocal::new(genius_service_url, genius_token)?;
        let musixmatch = Musixmatch::new(musixmatch_tokens)?;
        let lrclib = LrcLib::new()?;
        let azlyrics = AZLyrics::new(azlyrics_service_url)?;

        Ok(Self {
            genius,
            musixmatch,
            lrclib,
            azlyrics,
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
                        tracing::debug!("{} text not found", $name);
                    },
                };
            };
        }

        handle_provider!("Musixmatch", self.musixmatch);
        handle_provider!("LrcLib", self.lrclib);
        // handle_provider!("AZLyrics", self.azlyrics);
        handle_provider!("Genius", self.genius);

        Ok(None)
    }
}

#[derive(Debug)]
pub struct LyricsCacheManager {}

lazy_static! {
    static ref LYRICS_CACHE_TTL: RwLock<u64> = RwLock::new(24 * 60 * 60);
}

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
            *LYRICS_CACHE_TTL.read().await,
        )
        .await
    }
}
