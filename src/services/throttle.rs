use chrono::Duration;
use redis::AsyncCommands;

pub struct ThrottleService {}

impl ThrottleService {
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

        let duration = match count {
            // first minute (1min / 10s)
            0..6 => Duration::seconds(10),
            // next 10 minutes (6 + 10min / 15s)
            6..46 => Duration::seconds(15),
            // next 1 hour (46 + 1h / 20s)
            46..226 => Duration::seconds(20),
            // next 1 day (226 + 1d / 1min)
            226..1440 => Duration::minutes(1),
            // next 1 week (1440 + 1week / 3min)
            1440..4800 => Duration::minutes(3),
            // all time then
            4800.. => Duration::minutes(5),
        };

        Ok(duration)
    }

    fn get_key(user_id: &str) -> String {
        format!("rustify:spotify-throttle:{user_id}")
    }
}
