use chrono::Duration;
use redis::AsyncCommands;

pub struct SpotifyPollingBackoffService {}

impl SpotifyPollingBackoffService {
    pub async fn inc_idle(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<()> {
        let key = Self::get_key(user_id);
        let _: () = redis_conn.incr(key, 1).await?;

        Ok(())
    }

    pub async fn reset_idle(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<()> {
        let key = Self::get_key(user_id);
        let _: () = redis_conn.set(key, 0).await?;

        Ok(())
    }

    pub async fn get_idle(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<u64> {
        let key = Self::get_key(user_id);
        let count: Option<u64> = redis_conn.get(key).await?;

        Ok(count.unwrap_or_default())
    }

    pub async fn get_suspend_time(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Duration> {
        let count = Self::get_idle(redis_conn, user_id).await?;

        let interval_10s = Duration::seconds(10);
        let after_10_min = Duration::minutes(5).num_seconds() / interval_10s.num_seconds();
        if count < after_10_min as u64 {
            return Ok(interval_10s);
        }

        let interval_15s = Duration::seconds(15);
        let after_10_min =
            after_10_min + (Duration::minutes(10).num_seconds() / interval_15s.num_seconds());
        if count < after_10_min as u64 {
            return Ok(interval_15s);
        }

        let interval_20s = Duration::seconds(20);
        let after_1_hour =
            after_10_min + (Duration::hours(1).num_seconds() / interval_20s.num_seconds());
        if count < after_1_hour as u64 {
            return Ok(interval_20s);
        }

        let interval_1m = Duration::minutes(1);
        let after_1_day =
            after_1_hour + (Duration::days(1).num_seconds() / interval_1m.num_seconds());
        if count < after_1_day as u64 {
            return Ok(interval_1m);
        }

        let interval_3m = Duration::minutes(3);
        let after_1_week =
            after_1_day + (Duration::weeks(1).num_seconds() / interval_3m.num_seconds());
        if count < after_1_week as u64 {
            return Ok(interval_3m);
        }

        Ok(Duration::minutes(5))
    }

    fn get_key(user_id: &str) -> String {
        format!("rustify:spotify-polling-backoff:{user_id}")
    }
}
