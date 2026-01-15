use chrono::Duration;
use deadpool_redis::redis::AsyncCommands;

pub struct MagicService {}

impl MagicService {
    #[tracing::instrument(skip_all, fields(%user_id, %track_id))]
    pub async fn is_already_removed(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<bool> {
        let key = format!("rustify:magic:{user_id}:{track_id}");

        let already_removed: bool = redis_conn.exists(&key).await?;

        Ok(already_removed)
    }

    #[tracing::instrument(skip_all, fields(%user_id, %track_id))]
    pub async fn set_already_removed(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
        track_id: &str,
    ) -> anyhow::Result<()> {
        let key = format!("rustify:magic:{user_id}:{track_id}");

        let ttl = Duration::minutes(10).num_seconds() as u64;

        let _: () = redis_conn.set_ex(key, true, ttl).await?;

        Ok(())
    }
}
