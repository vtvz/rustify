use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_openai::config::{OPENAI_API_BASE, OpenAIConfig};
use rustrict::Replacements;
use sea_orm::{DatabaseConnection, DbConn, SqlxPostgresConnector};
use sqlx::postgres::PgConnectOptions;
use teloxide::Bot;
use teloxide::dispatching::dialogue::RedisStorage;
use teloxide::dispatching::dialogue::serializer::Bincode;

use crate::infrastructure::cache;
use crate::metrics::influx::InfluxClient;
use crate::services::UserService;
use crate::user::UserState;
use crate::{lyrics, profanity, spotify};

pub struct App {
    spotify_manager: spotify::Manager,
    lyrics: lyrics::Manager,
    bot: Bot,
    db: DatabaseConnection,
    influx: Option<InfluxClient>,
    redis: deadpool_redis::Pool,
    ai: Option<AIConfig>,
    dialogue_storage: Arc<RedisStorage<Bincode>>,
    server_http_address: String,
}

pub struct AIConfig {
    openai_client: async_openai::Client<OpenAIConfig>,
    model: String,
}

impl AIConfig {
    pub fn openai_client(&self) -> &async_openai::Client<OpenAIConfig> {
        &self.openai_client
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[derive(Deserialize, Debug)]
struct EnvConfig {
    telegram_bot_token: String,
    redis_url: String,
    database_url: String,

    spotify_id: String,
    spotify_secret: String,
    spotify_redirect_uri: Option<String>,

    musixmatch_user_tokens: Option<String>,
    genius_access_token: String,
    genius_service_url: String,
    lyrics_cache_ttl: Option<u64>,

    censor_blacklist: Option<String>,
    censor_whitelist: Option<String>,

    openai_api_key: Option<String>,
    openai_api_base: Option<String>,
    openai_api_model: Option<String>,

    influx_api_url: Option<String>,
    influx_token: Option<String>,
    influx_bucket: Option<String>,
    influx_org: Option<String>,
    influx_instance: Option<String>,

    server_http_address: Option<String>,
}

impl App {
    pub fn spotify_manager(&self) -> &spotify::Manager {
        &self.spotify_manager
    }

    pub fn lyrics(&self) -> &lyrics::Manager {
        &self.lyrics
    }

    pub fn bot(&self) -> &Bot {
        &self.bot
    }

    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    pub async fn redis_conn(&self) -> anyhow::Result<deadpool_redis::Connection> {
        Ok(self.redis.get().await?)
    }

    pub fn influx(&self) -> &Option<InfluxClient> {
        &self.influx
    }

    pub fn ai(&self) -> Option<&AIConfig> {
        self.ai.as_ref()
    }

    pub fn dialogue_storage(&self) -> &RedisStorage<Bincode> {
        &self.dialogue_storage
    }

    pub fn server_http_address(&self) -> &str {
        &self.server_http_address
    }
}

fn init_influx(env: &EnvConfig) -> anyhow::Result<Option<InfluxClient>> {
    let Some(api_url) = env.influx_api_url.clone() else {
        return Ok(None);
    };

    if api_url.is_empty() {
        return Ok(None);
    }

    let token = env
        .influx_token
        .clone()
        .context("Cannot obtain INFLUX_TOKEN variable")?;

    let bucket = env
        .influx_bucket
        .clone()
        .context("Cannot obtain INFLUX_BUCKET variable")?;

    let org = env
        .influx_org
        .clone()
        .context("Cannot obtain INFLUX_ORG variable")?;

    let instance_tag = env.influx_instance.as_deref();
    let client = InfluxClient::new(&api_url, &bucket, &org, &token, instance_tag)?;

    Ok(Some(client))
}

async fn init_db(env: &EnvConfig) -> anyhow::Result<DbConn> {
    let database_url = &env.database_url;

    let options = PgConnectOptions::from_str(database_url)?;

    // let options = options.pragma("key", "passphrase");

    let pool = sqlx::PgPool::connect_with(options)
        .await
        .context("Cannot connect DB")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Cannot migrate")?;

    Ok(SqlxPostgresConnector::from_sqlx_postgres_pool(pool))
}

async fn init_lyrics_manager(env: &EnvConfig, redis_url: &str) -> anyhow::Result<lyrics::Manager> {
    let mut musixmatch_tokens: Vec<_> = env
        .musixmatch_user_tokens
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(ToOwned::to_owned)
        .collect();

    if musixmatch_tokens.is_empty() {
        // https://github.com/spicetify/spicetify-cli/blob/7a9338db56719841fe4c431039dc2fbc287c0fe2/CustomApps/lyrics-plus/index.js#L64
        musixmatch_tokens.push("21051986b9886beabe1ce01c3ce94c96319411f8f2c122676365e3".to_owned());

        // https://github.com/spicetify/spicetify-cli/blob/045379c46ff4027d1db210da17a1e93f43941120/Extensions/popupLyrics.js#L276
        musixmatch_tokens.push("2005218b74f939209bda92cb633c7380612e14cb7fe92dcd6a780f".to_owned());
    }

    let genius_token = env.genius_access_token.clone();
    let genius_service_url = env.genius_service_url.clone();

    let default_ttl = chrono::Duration::hours(24).num_seconds() as u64;
    let lyrics_cache_ttl: u64 = env.lyrics_cache_ttl.unwrap_or(default_ttl);

    cache::CacheManager::init(redis_url.to_owned()).await;
    lyrics::LyricsCacheManager::init(lyrics_cache_ttl).await;
    lyrics::Manager::new(genius_service_url, genius_token, musixmatch_tokens)
}

fn init_rustrict(env: &EnvConfig) {
    env.censor_blacklist
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .for_each(profanity::Manager::add_word);

    env.censor_whitelist
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .for_each(profanity::Manager::remove_word);

    let mut r = Replacements::new();

    for b in b'a'..=b'z' {
        let c = b as char;
        r.insert(c, c); // still detect lowercased profanity.
        r.insert(c.to_ascii_uppercase(), c); // still detect capitalized profanity.
    }

    unsafe {
        *Replacements::customize_default() = r;
    }
}

async fn init_redis(redis_url: &str) -> anyhow::Result<deadpool_redis::Pool> {
    let cfg = deadpool_redis::Config::from_url(redis_url);
    let pool = cfg.create_pool(Some(deadpool_redis::Runtime::Tokio1))?;

    // Test the connection
    pool.get().await.context("Issue with connection")?;

    Ok(pool)
}

async fn init_ai(env: &EnvConfig) -> anyhow::Result<Option<AIConfig>> {
    let Some(api_key) = env.openai_api_key.as_deref() else {
        return Ok(None);
    };

    let openai_config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(env.openai_api_base.as_deref().unwrap_or(OPENAI_API_BASE));

    let http_client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(60))
        .build()?;

    let openai_client =
        async_openai::Client::with_config(openai_config).with_http_client(http_client);

    let config = AIConfig {
        openai_client,
        model: env.openai_api_model.clone().unwrap_or("gpt-4o".into()),
    };

    Ok(Some(config))
}

impl App {
    pub async fn init() -> anyhow::Result<&'static Self> {
        tracing::trace!("Init application");
        let env: EnvConfig = envy::from_env()?;

        let redis_url = &env.redis_url;
        let redis = init_redis(redis_url).await?;
        let spotify_manager = spotify::Manager::new(
            &env.spotify_id,
            &env.spotify_secret,
            env.spotify_redirect_uri.clone(),
        );
        let lyrics_manager = init_lyrics_manager(&env, redis_url).await?;
        let ai = init_ai(&env).await?;

        init_rustrict(&env);

        let bot = Bot::new(&env.telegram_bot_token);

        let db = init_db(&env).await?;

        let influx = init_influx(&env).context("Cannot configure Influx Client")?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app = Box::new(Self {
            bot,
            spotify_manager,
            lyrics: lyrics_manager,
            db,
            influx,
            redis,
            ai,
            dialogue_storage: RedisStorage::open(redis_url, Bincode).await?,
            server_http_address: env.server_http_address.unwrap_or("0.0.0.0:3000".into()),
        });

        let app = &*Box::leak(app);

        Ok(app)
    }

    pub async fn user_state(&'static self, user_id: &str) -> anyhow::Result<UserState> {
        let spotify = self.spotify_manager.for_user(&self.db, user_id).await?;
        let user = UserService::obtain_by_id(self.db(), user_id).await?;
        let state = UserState::new(user, spotify);

        Ok(state)
    }
}
