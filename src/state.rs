use std::str::FromStr;

use anyhow::Context;
use rspotify::AuthCodeSpotify;
use rspotify::clients::OAuthClient;
use rspotify::model::{PrivateUser, SubscriptionLevel};
use rustrict::Replacements;
use sea_orm::{DatabaseConnection, DbConn, SqlxPostgresConnector};
use sqlx::postgres::PgConnectOptions;
use teloxide::Bot;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::metrics::influx::InfluxClient;
use crate::{cache, lyrics, profanity, spotify, whitelist};

pub struct AppState {
    whitelist: whitelist::Manager,
    spotify_manager: spotify::Manager,
    lyrics: lyrics::Manager,
    bot: Bot,
    db: DatabaseConnection,
    influx: Option<InfluxClient>,
    redis: redis::Client,
}

impl AppState {
    pub fn whitelist(&self) -> &whitelist::Manager {
        &self.whitelist
    }

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

    pub async fn redis_conn(&self) -> anyhow::Result<redis::aio::MultiplexedConnection> {
        Ok(self.redis.get_multiplexed_tokio_connection().await?)
    }

    pub fn influx(&self) -> &Option<InfluxClient> {
        &self.influx
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

async fn init_lyrics_manager(redis_url: String) -> anyhow::Result<lyrics::Manager> {
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

    let azlyrics_service_url =
        dotenv::var("AZLYRICS_SERVICE_URL").context("Needs AZLYRICS_SERVICE_URL")?;

    let default_ttl = chrono::Duration::hours(24).num_seconds() as u64;
    let lyrics_cache_ttl: u64 = dotenv::var("LYRICS_CACHE_TTL")
        .unwrap_or(default_ttl.to_string())
        .parse()?;

    cache::CacheManager::init(redis_url).await;
    lyrics::LyricsCacheManager::init(lyrics_cache_ttl).await;
    lyrics::Manager::new(
        genius_service_url,
        genius_token,
        musixmatch_tokens,
        azlyrics_service_url,
    )
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

async fn init_redis(redis_url: &str) -> anyhow::Result<redis::Client> {
    let client = redis::Client::open(redis_url)?;

    client
        .get_multiplexed_tokio_connection()
        .await
        .context("Issue with connection")?;

    Ok(client)
}

impl AppState {
    pub async fn init() -> anyhow::Result<&'static Self> {
        tracing::trace!("Init application");

        let redis_url = dotenv::var("REDIS_URL").context("Need REDIS_URL variable")?;
        let redis = init_redis(&redis_url).await?;
        let spotify_manager = spotify::Manager::new();
        let lyrics_manager = init_lyrics_manager(redis_url).await?;

        init_rustrict();

        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = init_db().await?;

        let influx = init_influx().context("Cannot configure Influx Client")?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app = Box::new(Self {
            whitelist: whitelist::Manager::from_env(),
            bot,
            spotify_manager,
            lyrics: lyrics_manager,
            db,
            influx,
            redis,
        });

        let app = &*Box::leak(app);

        Ok(app)
    }

    pub async fn user_state(&'static self, user_id: &str) -> anyhow::Result<UserState> {
        let spotify = self.spotify_manager.for_user(&self.db, user_id).await?;
        let spotify = RwLock::new(spotify);

        let state = UserState {
            spotify,
            spotify_user: Default::default(),
            user_id: user_id.to_string(),
        };

        Ok(state)
    }
}

pub struct UserState {
    spotify: RwLock<AuthCodeSpotify>,
    user_id: String,

    spotify_user: Mutex<Option<Option<PrivateUser>>>,
}

impl UserState {
    pub async fn spotify(&self) -> RwLockReadGuard<'_, AuthCodeSpotify> {
        self.spotify.read().await
    }

    pub async fn spotify_write(&self) -> RwLockWriteGuard<'_, AuthCodeSpotify> {
        self.spotify.write().await
    }

    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    pub async fn is_spotify_authed(&self) -> bool {
        self.spotify()
            .await
            .token
            .lock()
            .await
            .expect("Failed to acquire lock")
            .is_some()
    }

    pub async fn spotify_user(&self) -> anyhow::Result<Option<PrivateUser>> {
        let mut lock = self.spotify_user.lock().await;

        if lock.is_none() {
            let user = if self.is_spotify_authed().await {
                let me = self.spotify().await.me().await?;

                Some(me)
            } else {
                None
            };

            lock.replace(user);
        }

        Ok(lock.as_ref().context("Should be initialized")?.clone())
    }

    pub async fn is_spotify_premium(&self) -> anyhow::Result<bool> {
        let res = self
            .spotify_user()
            .await?
            .map(|spotify_user| {
                spotify_user
                    .product
                    .map(|product| product == SubscriptionLevel::Premium)
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(res)
    }
}
