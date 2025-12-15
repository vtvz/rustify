use chrono::Duration;
use redis::AsyncCommands;

pub enum RateLimitOutput {
    Allowed,
    NeedToWait(Duration),
}

pub struct RateLimitService {}

impl RateLimitService {
    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn track_analyze(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(
            redis_conn,
            user_id,
            "track_analyze",
            1,
            Duration::minutes(5),
        )
        .await
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn track_details(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(
            redis_conn,
            user_id,
            "track_details",
            1,
            Duration::seconds(15),
        )
        .await
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn track_dislike(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(
            redis_conn,
            user_id,
            "track_dislike",
            1,
            Duration::seconds(10),
        )
        .await
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn track_like(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(redis_conn, user_id, "track_like", 1, Duration::seconds(10)).await
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn recommendasion(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(redis_conn, user_id, "recommendasion", 1, Duration::hours(1)).await
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn magic_playlist(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<RateLimitOutput> {
        Self::rate_limit(redis_conn, user_id, "magic_playlist", 1, Duration::hours(6)).await
    }

    #[tracing::instrument(skip_all, fields(user_id, func))]
    async fn rate_limit(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
        func: &str,
        max_attempts: u32,
        time_window: Duration,
    ) -> anyhow::Result<RateLimitOutput> {
        let key = format!("rustify:ratelimit:{user_id}:{func}");

        let ttl_seconds = time_window.num_seconds();

        let count: u32 = redis_conn.incr(&key, 1).await?;

        if count == 1 {
            let _: () = redis_conn.expire(&key, ttl_seconds).await?;
        }

        if count > max_attempts {
            let remaining_ttl: i64 = redis_conn.ttl(&key).await?;

            let wait_duration = if remaining_ttl > 0 {
                Duration::seconds(remaining_ttl)
            } else {
                Duration::zero()
            };

            return Ok(RateLimitOutput::NeedToWait(wait_duration));
        }

        Ok(RateLimitOutput::Allowed)
    }
}
