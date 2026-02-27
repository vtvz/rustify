use chrono::Duration;
use redis::AsyncCommands as _;

use crate::spotify::ShortTrack;

pub struct SoulOverAIProvider {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize)]
struct AIArtist {
    // name: String,
    spotify: Option<String>,
}

const REDIS_KEY_POPULATED: &str = "rustify:ai_slop:soul_over_ai:populated";
const REDIS_KEY_ARTIST_PREFIX: &str = "rustify:ai_slop:soul_over_ai:artist";

impl SoulOverAIProvider {
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
        }
    }

    async fn ensure_populated(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
    ) -> anyhow::Result<()> {
        let exists: bool = redis_conn.exists(REDIS_KEY_POPULATED).await?;

        if exists {
            return Ok(());
        }

        self.populate(redis_conn).await?;

        let _: () = redis_conn
            .set_ex(
                REDIS_KEY_POPULATED,
                1,
                (Duration::days(1) - Duration::minutes(10)).num_seconds() as _,
            )
            .await?;

        Ok(())
    }

    async fn populate(&self, redis_conn: &mut deadpool_redis::Connection) -> anyhow::Result<()> {
        tracing::trace!("Populating soul-over-ai DB of AI slop");

        let res = self
            .client
            .get("https://raw.githubusercontent.com/xoundbyte/soul-over-ai/refs/heads/main/dist/artists.json")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let artists: Vec<AIArtist> = serde_json::from_reader(res.as_ref())?;

        for artist in artists {
            let Some(id) = artist.spotify else {
                continue;
            };

            let _: () = redis_conn
                .set_ex(
                    format!("{REDIS_KEY_ARTIST_PREFIX}:{id}"),
                    1,
                    Duration::days(1).num_seconds() as _,
                )
                .await?;
        }

        Ok(())
    }

    async fn is_artist_ai(
        redis_conn: &mut deadpool_redis::Connection,
        artist_id: &str,
    ) -> anyhow::Result<bool> {
        let exists: bool = redis_conn
            .exists(format!("{REDIS_KEY_ARTIST_PREFIX}:{artist_id}"))
            .await?;

        Ok(exists)
    }

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

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        Self::ensure_populated(self, redis_conn).await?;

        self.any_artist_ai(redis_conn, &track.artist_ids()).await
    }
}

impl Default for SoulOverAIProvider {
    fn default() -> Self {
        Self::new()
    }
}
