use std::sync::LazyLock;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct CacheManager {}

static REDIS_URL: LazyLock<RwLock<String>> = LazyLock::new(|| RwLock::new(String::new()));

impl CacheManager {
    pub async fn init(redis_url: String) {
        let mut lock = REDIS_URL.write().await;
        *lock = redis_url;
    }

    pub async fn redis_cached_build<T>(
        namespace: &str,
        cache_ttl: Duration,
    ) -> anyhow::Result<cached::AsyncRedisCache<String, T>>
    where
        T: Sync + Send + Serialize + DeserializeOwned,
    {
        let res = cached::AsyncRedisCache::new(format!("rustify:{namespace}:"), cache_ttl)
            .set_refresh(true)
            .set_connection_string(REDIS_URL.read().await.as_ref())
            .set_namespace("")
            .build()
            .await;

        Ok(res?)
    }
}
