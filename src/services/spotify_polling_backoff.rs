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

    pub async fn get_idle_ticks(
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
        let count = Self::get_idle_ticks(redis_conn, user_id).await?;

        #[rustfmt::skip]
        let intervals = [
            ( Duration::minutes(1),  Duration::seconds(5)  ),
            ( Duration::minutes(5),  Duration::seconds(10) ),
            ( Duration::minutes(10), Duration::seconds(15) ),
            ( Duration::hours(1),    Duration::seconds(20) ),
            ( Duration::days(1),     Duration::seconds(30) ),
            ( Duration::days(3),     Duration::minutes(1)  ),
            ( Duration::weeks(1),    Duration::minutes(3)  ),
        ];

        let mut accumulated_ticks = 0u64;

        for (period, interval) in intervals {
            let ticks_in_period = period.num_seconds() / interval.num_seconds();
            accumulated_ticks += ticks_in_period as u64;

            if count < accumulated_ticks {
                return Ok(interval);
            }
        }

        Ok(Duration::minutes(5))
    }

    fn get_key(user_id: &str) -> String {
        format!("rustify:spotify-polling-backoff:{user_id}")
    }
}
