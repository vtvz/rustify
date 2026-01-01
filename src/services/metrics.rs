use deadpool_redis::redis::AsyncCommands as _;

pub struct MetricsService {}

impl MetricsService {
    pub async fn spotify_429_inc(
        redis_conn: &mut deadpool_redis::Connection,
    ) -> anyhow::Result<()> {
        let key = Self::spotify_429_key();
        let _: () = redis_conn.incr(key, 1).await?;

        Ok(())
    }

    pub async fn spotify_429_get(
        redis_conn: &mut deadpool_redis::Connection,
    ) -> anyhow::Result<u64> {
        let key = Self::spotify_429_key();
        let count: Option<u64> = redis_conn.get(key).await?;

        Ok(count.unwrap_or_default())
    }

    fn spotify_429_key() -> &'static str {
        "rustify:metrics:spotify-429"
    }
}
