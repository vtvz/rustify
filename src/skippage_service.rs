use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;

pub struct SkippageService {}

impl SkippageService {
    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn get_current_playing(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
    ) -> anyhow::Result<String> {
        let playing_key = format!("rustify:skipage:{}:playing", user_id);

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
        let playing_key = format!("rustify:skipage:{}:playing", user_id);
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
        let track_key = format!("rustify:skipage:{}:{}", user_id, track_id);

        let _: () = redis_conn.set_ex(&track_key, 1, skippage_secs).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn get_track_played(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<bool> {
        let track_key = format!("rustify:skipage:{}:{}", user_id, track_id);

        let track_exists: bool = redis_conn.exists(&track_key).await?;

        Ok(track_exists)
    }

    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn delete_skipage_entries(
        redis_conn: &mut MultiplexedConnection,
        user_id: &str,
    ) -> anyhow::Result<u64> {
        // Use SCAN to find all keys matching the pattern
        let pattern = format!("rustify:skipage:{user_id}:*");
        let mut cursor = 0;
        let mut total_deleted = 0u64;

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
                let deleted: u64 = redis_conn.del(&keys).await?;
                total_deleted += deleted;
            }

            cursor = new_cursor;

            if cursor == 0 {
                break;
            }
        }

        tracing::info!(
            total_deleted = total_deleted,
            user_id = user_id,
            "Completed deletion of skipage entries"
        );

        Ok(total_deleted)
    }
}
