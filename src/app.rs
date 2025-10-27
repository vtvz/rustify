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
}

fn init_influx() -> anyhow::Result<Option<InfluxClient>> {
    let Ok(api_url) = dotenv::var("INFLUX_API_URL") else {
        return Ok(None);
    };

    if api_url.is_empty() {
        return Ok(None);
    }

    let token = dotenv::var("INFLUX_TOKEN").context("Cannot obtain INFLUX_TOKEN variable")?;
    let bucket = dotenv::var("INFLUX_BUCKET").context("Cannot obtain INFLUX_BUCKET variable")?;
    let org = dotenv::var("INFLUX_ORG").context("Cannot obtain INFLUX_ORG variable")?;

    let instance_tag = dotenv::var("INFLUX_INSTANCE").ok();
    let client = InfluxClient::new(&api_url, &bucket, &org, &token, instance_tag.as_deref())?;

    Ok(Some(client))
}

async fn init_db() -> anyhow::Result<DbConn> {
    let database_url = dotenv::var("DATABASE_URL").context("Needs DATABASE_URL")?;

    let options = PgConnectOptions::from_str(&database_url)?;

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

async fn init_lyrics_manager(redis_url: &str) -> anyhow::Result<lyrics::Manager> {
    let mut musixmatch_tokens: Vec<_> = dotenv::var("MUSIXMATCH_USER_TOKENS")
        .unwrap_or_else(|_| "".into())
        .split(',')
        .map(ToOwned::to_owned)
        .collect();

    if musixmatch_tokens.is_empty() {
        // https://github.com/spicetify/spicetify-cli/blob/7a9338db56719841fe4c431039dc2fbc287c0fe2/CustomApps/lyrics-plus/index.js#L64
        musixmatch_tokens.push("21051986b9886beabe1ce01c3ce94c96319411f8f2c122676365e3".to_owned());

        // https://github.com/spicetify/spicetify-cli/blob/045379c46ff4027d1db210da17a1e93f43941120/Extensions/popupLyrics.js#L276
        musixmatch_tokens.push("2005218b74f939209bda92cb633c7380612e14cb7fe92dcd6a780f".to_owned());
    }

    let genius_token = dotenv::var("GENIUS_ACCESS_TOKEN").context("Needs GENIUS_ACCESS_TOKEN")?;
    let genius_service_url =
        dotenv::var("GENIUS_SERVICE_URL").context("Needs GENIUS_SERVICE_URL")?;

    let default_ttl = chrono::Duration::hours(24).num_seconds() as u64;
    let lyrics_cache_ttl: u64 = dotenv::var("LYRICS_CACHE_TTL")
        .unwrap_or(default_ttl.to_string())
        .parse()?;

    cache::CacheManager::init(redis_url.to_owned()).await;
    lyrics::LyricsCacheManager::init(lyrics_cache_ttl).await;
    lyrics::Manager::new(genius_service_url, genius_token, musixmatch_tokens)
}

fn init_rustrict() {
    dotenv::var("CENSOR_BLACKLIST")
        .unwrap_or_default()
        .split(',')
        .for_each(profanity::Manager::add_word);

    dotenv::var("CENSOR_WHITELIST")
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

async fn init_ai() -> anyhow::Result<Option<AIConfig>> {
    let Ok(api_key) = dotenv::var("OPENAI_API_KEY") else {
        return Ok(None);
    };

    let openai_config = OpenAIConfig::new()
        .with_api_key(api_key)
        .with_api_base(dotenv::var("OPENAI_API_BASE").unwrap_or(OPENAI_API_BASE.into()));

    let http_client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()?;

    let openai_client =
        async_openai::Client::with_config(openai_config).with_http_client(http_client);

    let config = AIConfig {
        openai_client,
        model: dotenv::var("OPENAI_API_MODEL").unwrap_or("gpt-4o".into()),
    };

    Ok(Some(config))
}

impl App {
    pub async fn init() -> anyhow::Result<&'static Self> {
        tracing::trace!("Init application");

        let redis_url = dotenv::var("REDIS_URL").context("Need REDIS_URL variable")?;
        let redis = init_redis(&redis_url).await?;
        let spotify_manager = spotify::Manager::new();
        let lyrics_manager = init_lyrics_manager(&redis_url).await?;
        let ai = init_ai().await?;

        init_rustrict();

        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = init_db().await?;

        let influx = init_influx().context("Cannot configure Influx Client")?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app = Box::new(Self {
            bot,
            spotify_manager,
            lyrics: lyrics_manager,
            db,
            influx,
            redis,
            ai,
            dialogue_storage: RedisStorage::open(&redis_url, Bincode).await?,
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
