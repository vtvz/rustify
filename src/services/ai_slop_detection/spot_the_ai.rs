use chrono::Duration;
use redis::AsyncCommands as _;

use crate::spotify::ShortTrack;

pub struct SpotTheAIProvider {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize)]
struct AIArtists {
    artists: Vec<String>,
}

const REDIS_KEY_POPULATED: &str = "rustify:ai_slop:spot_the_ai:populated";
const REDIS_KEY_ARTIST_PREFIX: &str = "rustify:ai_slop:spot_the_ai:artist";

impl SpotTheAIProvider {
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
        tracing::trace!("Populating spot-the-ai DB of AI slop");

        let res = self
            .client
            .get("https://spot-the-ai.com/api/list/")
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        let artists: AIArtists = serde_json::from_reader(res.as_ref())?;

        for artist_name in artists.artists {
            let _: () = redis_conn
                .set_ex(
                    format!("{REDIS_KEY_ARTIST_PREFIX}:{:?}", md5::compute(artist_name)),
                    1,
                    Duration::days(1).num_seconds() as _,
                )
                .await?;
        }

        Ok(())
    }

    async fn is_artist_ai(
        redis_conn: &mut deadpool_redis::Connection,
        artist_name: &str,
    ) -> anyhow::Result<bool> {
        let exists: bool = redis_conn
            .exists(format!(
                "{REDIS_KEY_ARTIST_PREFIX}:{:?}",
                md5::compute(artist_name)
            ))
            .await?;

        Ok(exists)
    }

    async fn any_artist_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        artist_names: &[&str],
    ) -> anyhow::Result<bool> {
        for artist_name in artist_names {
            if Self::is_artist_ai(redis_conn, artist_name).await? {
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

        self.any_artist_ai(redis_conn, &track.artist_names()).await
    }
}

impl Default for SpotTheAIProvider {
    fn default() -> Self {
        Self::new()
    }
}
