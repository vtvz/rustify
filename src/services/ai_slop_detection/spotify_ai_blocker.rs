use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use async_trait::async_trait;
use chrono::Duration;
use redis::AsyncCommands as _;

use crate::services::ai_slop_detection::{AISlopDetectionPrediction, AISlopDetector};
use crate::spotify::ShortTrack;

pub struct SpotifyAIBlockerProvider {
    client: reqwest::Client,
    populating: AtomicBool,
}

#[derive(Debug, serde::Deserialize)]
struct AIArtist {
    // artist: String,
    id: String,
}

const REDIS_KEY_POPULATED: &str = "rustify:ai_slop:spotify_ai_blocker:populated";
const REDIS_KEY_ARTIST_PREFIX: &str = "rustify:ai_slop:spotify_ai_blocker:artist";

const RETRY_DELAY: Duration = Duration::milliseconds(100);
const POPULATE_TIMEOUT: Duration = Duration::seconds(20);

impl SpotifyAIBlockerProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(
                    Duration::seconds(10)
                        .to_std()
                        .expect("It's positive. Will work"),
                )
                .build()
                .expect("Should work"),
            populating: AtomicBool::new(false),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn ensure_populated(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
    ) -> anyhow::Result<()> {
        if redis_conn.exists(REDIS_KEY_POPULATED).await? {
            return Ok(());
        }

        if self
            .populating
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            let result = self.populate(redis_conn).await;

            let result = if result.is_ok() {
                redis_conn
                    .set_ex(
                        REDIS_KEY_POPULATED,
                        1,
                        (Duration::days(1) - Duration::minutes(10)).num_seconds() as _,
                    )
                    .await
                    .map_err(Into::into)
            } else {
                result
            };

            self.populating.store(false, Ordering::SeqCst);

            return result;
        }

        let deadline = Instant::now() + POPULATE_TIMEOUT.to_std().expect("positive duration");

        while Instant::now() < deadline {
            tokio::time::sleep(RETRY_DELAY.to_std().expect("positive duration")).await;

            if !self.populating.load(Ordering::SeqCst) {
                if redis_conn.exists(REDIS_KEY_POPULATED).await? {
                    return Ok(());
                }

                anyhow::bail!("Population failed by another task");
            }
        }

        anyhow::bail!("Timeout waiting for population to complete")
    }

    #[tracing::instrument(skip_all)]
    async fn populate(&self, redis_conn: &mut deadpool_redis::Connection) -> anyhow::Result<()> {
        tracing::trace!("Populating spotify-ai-blocker DB of AI slop");

        let res = self
            .client
            .get("https://github.com/CennoxX/spotify-ai-blocker/raw/refs/heads/main/SpotifyAiArtists.csv")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let mut rdr = csv::Reader::from_reader(res.as_ref());

        let expiry_seconds = Duration::days(1).num_seconds() as u64;
        let mut pipe = deadpool_redis::redis::Pipeline::new();

        for result in rdr.deserialize() {
            let record: AIArtist = result?;

            pipe.cmd("SETEX")
                .arg(format!("{REDIS_KEY_ARTIST_PREFIX}:{}", record.id))
                .arg(expiry_seconds)
                .arg(1)
                .ignore();
        }

        let _: () = pipe.query_async(redis_conn).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(%artist_id))]
    async fn is_artist_ai(
        redis_conn: &mut deadpool_redis::Connection,
        artist_id: &str,
    ) -> anyhow::Result<bool> {
        let exists: bool = redis_conn
            .exists(format!("{REDIS_KEY_ARTIST_PREFIX}:{artist_id}"))
            .await?;

        Ok(exists)
    }

    #[tracing::instrument(skip_all, fields(artist_ids = artist_ids.join(", ")))]
    async fn any_artist_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        artist_ids: &[&str],
    ) -> anyhow::Result<bool> {
        for artist_id in artist_ids {
            if Self::is_artist_ai(redis_conn, artist_id).await? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    #[tracing::instrument(skip_all, fields(track_id = %track.id()))]
    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        Self::ensure_populated(self, redis_conn).await?;

        self.any_artist_ai(redis_conn, &track.artist_ids()).await
    }
}

#[async_trait]
impl AISlopDetector for SpotifyAIBlockerProvider {
    #[tracing::instrument(skip_all, fields(track_id = %track.id()))]
    async fn detect(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<AISlopDetectionPrediction> {
        self.is_track_ai(redis_conn, track).await.map(|res| {
            if res {
                AISlopDetectionPrediction::PureAI
            } else {
                AISlopDetectionPrediction::HumanMade
            }
        })
    }
}

impl Default for SpotifyAIBlockerProvider {
    fn default() -> Self {
        Self::new()
    }
}
