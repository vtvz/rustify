use chrono::Duration;
use deadpool_redis::redis::AsyncCommands;
use itertools::Itertools;

use crate::spotify::ShortTrack;

pub struct RecommendasionService {}

impl RecommendasionService {
    #[tracing::instrument(skip_all, fields(user_id))]
    pub async fn get_already_recommended(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
    ) -> anyhow::Result<Vec<ShortTrack>> {
        let recommended_key = format!("rustify:recommendasion:{user_id}:recommended");

        let recommended: Option<String> = redis_conn.get(&recommended_key).await?;

        let recommended = recommended.unwrap_or_default();

        let recommended: Vec<ShortTrack> = serde_json::from_str(&recommended).unwrap_or_default();

        Ok(recommended)
    }

    #[tracing::instrument(skip_all, fields(user_id, track_id))]
    pub async fn save_already_recommended(
        redis_conn: &mut deadpool_redis::Connection,
        user_id: &str,
        recommended: &[ShortTrack],
    ) -> anyhow::Result<()> {
        let recommended_key = format!("rustify:recommendasion:{user_id}:recommended");

        let recommended = recommended.iter().take(1000).collect_vec();

        let ttl = Duration::days(30);

        let _: () = redis_conn
            .set_ex(
                recommended_key,
                serde_json::to_string(&recommended)?,
                ttl.num_seconds() as _,
            )
            .await?;

        Ok(())
    }
}
