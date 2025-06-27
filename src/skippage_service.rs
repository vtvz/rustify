use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;

pub struct SkippageService {}

impl SkippageService {
    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn get_current_playing(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
    ) -> anyhow::Result<String> {
        let playing_key = format!("rustify:skippage:{}:playing", user_id);

        let current_playing: Option<String> = redis_conn.get(&playing_key).await?;
        let current_playing = current_playing.unwrap_or_else(|| String::from("nothing"));

        Ok(current_playing)
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn save_current_playing(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<()> {
        let playing_key = format!("rustify:skippage:{}:playing", user_id);
        let _: () = redis_conn.set(&playing_key, track_id).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn save_track_played(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
        track_id: &str,
        skippage_secs: u64,
    ) -> anyhow::Result<()> {
        let track_key = format!("rustify:skippage:{}:{}", user_id, track_id);

        tracing::debug!("skippage secs {} {}", skippage_secs, track_key);

        let _: () = redis_conn.set_ex(&track_key, 1, skippage_secs).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn get_track_played(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<bool> {
        let track_key = format!("rustify:skippage:{}:{}", user_id, track_id);

        let track_exists: bool = redis_conn.exists(&track_key).await?;

        Ok(track_exists)
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn update_skippage_entries_ttl(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
        skippage_duration: chrono::Duration,
    ) -> anyhow::Result<()> {
        let skippage_secs = skippage_duration.num_seconds();
        let pattern = format!("rustify:skippage:{user_id}:*");
        let mut cursor = 0;

        loop {
            // SCAN with pattern matching
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(100) // Process in batches of 100
            .query_async(redis_conn)
            .await?;

            if !keys.is_empty() {
                for key in &keys {
                    let ttl: i64 = redis_conn.ttl(key).await?;

                    tracing::debug!("update {} {}", skippage_secs, key);

                    if ttl > skippage_secs {
                        let _: () = redis_conn.expire(key, skippage_secs).await?;
                    }
                }
            }

            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}
