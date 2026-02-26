use chrono::Duration;
use redis::AsyncCommands as _;

use crate::spotify::ShortTrack;

pub struct SpotifyAIBlockerProvider {
    client: reqwest::Client,
}

#[derive(Debug, serde::Deserialize)]
struct AIArtist {
    // artist: String,
    id: String,
}

const REDIS_KEY_POPULATED: &str = "rustify:ai_slop:spotify_ai_blocker:populated";
const REDIS_KEY_ARTIST_PREFIX: &str = "rustify:ai_slop:spotify_ai_blocker:artist";

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
        }
    }

    pub async fn ensure_populated(
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

        for result in rdr.deserialize() {
            let record: AIArtist = result?;

            let _: () = redis_conn
                .set_ex(
                    format!("{REDIS_KEY_ARTIST_PREFIX}:{}", record.id),
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
        Self::ensure_populated(self, redis_conn).await?;

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
        self.any_artist_ai(redis_conn, &track.artist_ids()).await
    }
}

impl Default for SpotifyAIBlockerProvider {
    fn default() -> Self {
        Self::new()
    }
}
