use deadpool_redis::redis::AsyncCommands;
use genius::GeniusLocal;
use isolang::Language;
use lrclib::LrcLib;
use musixmatch::Musixmatch;
use serde::Serialize;
use strum_macros::Display;

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

#[derive(derive_more::From, Serialize, Deserialize)]
pub enum SearchResultData {
    Musixmatch(musixmatch::Lyrics),
    Genius(genius::SearchResult),
    LrcLib(lrclib::SearchResult),
}

impl SearchResultData {
    #[must_use]
    pub fn as_search_result(&self) -> &dyn SearchResult {
        match self {
            SearchResultData::Musixmatch(result) => result,
            SearchResultData::Genius(result) => result,
            SearchResultData::LrcLib(result) => result,
        }
    }
}

impl SearchResult for SearchResultData {
    fn provider(&self) -> Provider {
        self.as_search_result().provider()
    }

    fn lyrics(&self) -> Vec<&str> {
        self.as_search_result().lyrics()
    }

    fn link(&self) -> String {
        self.as_search_result().link()
    }

    fn link_text(&self, full: bool) -> String {
        self.as_search_result().link_text(full)
    }

    fn language(&self) -> Language {
        self.as_search_result().language()
    }

    fn line_index_name(&self, index: usize) -> String {
        self.as_search_result().line_index_name(index)
    }
}

pub struct Manager {
    genius: GeniusLocal,
    musixmatch: Musixmatch,
    lrclib: LrcLib,

    lyrics_cache_ttl: u64,
}

impl Manager {
    pub fn new(
        genius_service_url: String,
        genius_token: String,
        musixmatch_tokens: impl IntoIterator<Item = String>,
        lyrics_cache_ttl: u64,
    ) -> anyhow::Result<Self> {
        let genius = GeniusLocal::new(genius_service_url, genius_token)?;
        let musixmatch = Musixmatch::new(musixmatch_tokens)?;
        let lrclib = LrcLib::new()?;

        Ok(Self {
            genius,
            musixmatch,
            lrclib,
            lyrics_cache_ttl,
        })
    }

    #[tracing::instrument(skip_all, fields(%track_id))]
    pub async fn set_track_cache(
        redis_conn: &mut deadpool_redis::Connection,
        track_id: &str,
        data: &SearchResultData,
        lyrics_cache_ttl: u64,
    ) -> anyhow::Result<()> {
        let track_key = format!("rustify:lyrics:{track_id}");

        let _: () = redis_conn
            .set_ex(&track_key, serde_json::to_string(data)?, lyrics_cache_ttl)
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(%track_id))]
    pub async fn get_track_cache(
        redis_conn: &mut deadpool_redis::Connection,
        track_id: &str,
    ) -> anyhow::Result<Option<SearchResultData>> {
        let track_key = format!("rustify:lyrics:{track_id}");

        let data: Option<String> = redis_conn.get(&track_key).await?;

        let Some(data) = data else {
            return Ok(None);
        };

        let data: Option<SearchResultData> = serde_json::from_str(&data).ok();

        Ok(data)
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
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<SearchResultData>> {
        if let Some(data) = Self::get_track_cache(redis_conn, track.id()).await? {
            return Ok(Some(data));
        }

        // tired to fight type system to handle this with vec
        macro_rules! handle_provider {
            ($name:expr, $provider:expr) => {
                let result = $provider.search_for_track(track).await;

                match result {
                    Ok(Some(res)) => {
                        let data = res.into();

                        Self::set_track_cache(redis_conn, track.id(), &data, self.lyrics_cache_ttl).await?;

                        return Ok(Some(data));
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
