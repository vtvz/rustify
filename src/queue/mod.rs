use std::str::FromStr;

use apalis::prelude::MakeShared;
use apalis_redis::shared::SharedRedisStorage;
use apalis_redis::{RedisConfig, RedisStorage};
use redis::aio::MultiplexedConnection;

pub mod profanity_check;

pub struct QueueManager {
    #[allow(dead_code)]
    storage: SharedRedisStorage,

    profanity_queue: RedisStorage<profanity_check::ProfanityCheckQueueTask, MultiplexedConnection>,
}

impl QueueManager {
    #[must_use]
    pub fn profanity_queue(
        &self,
    ) -> RedisStorage<profanity_check::ProfanityCheckQueueTask, MultiplexedConnection> {
        self.profanity_queue.clone()
    }

    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let mut conn_info = redis::ConnectionInfo::from_str(redis_url)?;
        conn_info.redis.protocol = redis::ProtocolVersion::RESP3;
        let client = redis::Client::open(conn_info)?;

        let mut storage = SharedRedisStorage::new(client).await?;

        let profanity_queue = storage.make_shared_with_config(
            RedisConfig::default().set_namespace("rustify:profanity_check"),
        )?;

        Ok(Self {
            storage,
            profanity_queue,
        })
    }
}
