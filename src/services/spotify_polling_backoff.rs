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

    pub async fn get_last_activity(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<i64> {
        let key = Self::get_key(user_id);
        let last_activity: Result<Option<i64>, _> = redis_conn.get(key).await;

        // Protection from trash in the db
        let last_activity = last_activity.unwrap_or_default();

        if last_activity.is_none() {
            Self::update_activity(redis_conn, user_id).await?;
        }

        Ok(last_activity.unwrap_or_else(|| Clock::now().and_utc().timestamp()))
    }

    pub async fn get_idle_duration(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Duration> {
        let now = Clock::now().and_utc().timestamp();

        let last_activity = Self::get_last_activity(redis_conn, user_id).await?;

        let dur = Duration::seconds(now - last_activity);

        Ok(dur)
    }

    pub async fn get_suspend_time(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Option<Duration>> {
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
                return Ok(Some(interval));
            }
        }

        Ok(None)
    }

    fn get_key(user_id: &str) -> String {
        format!("rustify:spotify-polling-backoff:{user_id}")
    }
}
