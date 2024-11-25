use azlyrics::AZLyrics;
use genius::GeniusLocal;
use isolang::Language;
use lazy_static::lazy_static;
use lrclib::LrcLib;
use musixmatch::Musixmatch;
use rspotify::model::FullTrack;
use serde::Serialize;
use serde::de::DeserializeOwned;
use strum_macros::Display;
use tokio::sync::RwLock;

use crate::spotify;

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
            track_id = %spotify::utils::get_track_id(track),
            track_name = %spotify::utils::create_track_name(track),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &FullTrack,
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
        handle_provider!("AZLyrics", self.azlyrics);
        handle_provider!("Genius", self.genius);

        Ok(None)
    }
}

#[derive(Debug)]
pub struct LyricsCacheManager {}

lazy_static! {
    static ref REDIS_URL: RwLock<String> = RwLock::new(String::new());
    static ref LYRICS_CACHE_TTL: RwLock<u64> = RwLock::new(24 * 60 * 60);
}

impl LyricsCacheManager {
    pub async fn init(redis_url: String, lyrics_cache_ttl: u64) {
        let mut lock = REDIS_URL.write().await;
        *lock = redis_url;

        let mut lock = LYRICS_CACHE_TTL.write().await;
        *lock = lyrics_cache_ttl;
    }

    pub async fn redis_cache_build<T: Sync + Send + Serialize + DeserializeOwned>(
        provider: &str,
    ) -> anyhow::Result<cached::AsyncRedisCache<String, T>> {
        let res = cached::AsyncRedisCache::new(
            format!("rustify:lyrics:{provider}:"),
            *LYRICS_CACHE_TTL.read().await,
        )
        .set_refresh(true)
        .set_connection_string(REDIS_URL.read().await.as_ref())
        .set_namespace("")
        .build()
        .await;

        Ok(res?)
    }
}
