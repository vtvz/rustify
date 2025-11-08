use chrono::Duration;
use redis::AsyncCommands;

use crate::utils::Clock;

pub struct SpotifyPollingBackoffService {}

impl SpotifyPollingBackoffService {
    pub async fn update_activity(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<()> {
        let key = Self::get_key(user_id);
        let now = Clock::now().and_utc().timestamp();
        let _: () = redis_conn.set(key, now).await?;

        Ok(())
    }

    pub async fn get_idle_duration(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Duration> {
        let key = Self::get_key(user_id);
        let last_activity: Option<i64> = redis_conn.get(key).await?;
        let now = Clock::now().and_utc().timestamp();

        let dur = Duration::seconds(now - last_activity.unwrap_or(now));

        Ok(dur)
    }

    pub async fn get_suspend_time(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Duration> {
        let idle_duration = Self::get_idle_duration(redis_conn, user_id).await?;

        #[rustfmt::skip]
        let intervals = [
            ( Duration::minutes(1),  Duration::seconds(6)  ),
            ( Duration::minutes(5),  Duration::seconds(9) ),
            ( Duration::minutes(10), Duration::seconds(15) ),
            ( Duration::hours(1),    Duration::seconds(21) ),
            ( Duration::days(1),     Duration::seconds(30) ),
            ( Duration::days(3),     Duration::minutes(1)  ),
            ( Duration::weeks(1),    Duration::minutes(3)  ),
        ];

        for (period, interval) in intervals {
            if idle_duration < period {
                return Ok(interval);
            }
        }

        Ok(Duration::minutes(5))
    }

    fn get_key(user_id: &str) -> String {
        format!("rustify:spotify-polling-backoff:{user_id}")
    }
}
