use std::str::FromStr as _;

use apalis::prelude::MakeShared as _;
use apalis_redis::shared::SharedRedisStorage;
use apalis_redis::{RedisConfig, RedisStorage};
use redis::aio::MultiplexedConnection;

pub mod track_check;

pub struct QueueManager {
    #[allow(dead_code)]
    storage: SharedRedisStorage,

    track_check_queue: RedisStorage<track_check::TrackCheckQueueTask, MultiplexedConnection>,
}

impl QueueManager {
    #[must_use]
    pub fn track_check_queue(
        &self,
    ) -> RedisStorage<track_check::TrackCheckQueueTask, MultiplexedConnection> {
        self.track_check_queue.clone()
    }

    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let mut conn_info = redis::ConnectionInfo::from_str(redis_url)?;
        conn_info.redis.protocol = redis::ProtocolVersion::RESP3;
        let client = redis::Client::open(conn_info)?;

        let mut storage = SharedRedisStorage::new(client).await?;

        let profanity_queue = storage
            .make_shared_with_config(RedisConfig::default().set_namespace("rustify:track_check"))?;

        Ok(Self {
            storage,
            track_check_queue: profanity_queue,
        })
    }
}
