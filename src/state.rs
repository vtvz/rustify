use anyhow::Context;
use rspotify::clients::OAuthClient;
use rspotify::model::{PrivateUser, SubscriptionLevel};
use rspotify::AuthCodeSpotify;
use sea_orm::{DatabaseConnection, DbConn, SqlxSqliteConnector};
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
use teloxide::Bot;
use tokio::sync::RwLock;

use crate::{lyrics, profanity, spotify};

pub struct AppState {
    pub spotify_manager: spotify::Manager,
    pub lyrics: lyrics::Manager,
    pub bot: Bot,
    pub db: DatabaseConnection,
}

async fn db() -> anyhow::Result<DbConn> {
    let database_url = dotenv::var("DATABASE_URL").context("Needs DATABASE_URL")?;

    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);

    // let options = options.pragma("key", "passphrase");

    let pool = sqlx::SqlitePool::connect_with(options)
        .await
        .context("Cannot connect DB")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Cannot migrate")?;

    Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

fn lyrics_manager() -> anyhow::Result<lyrics::Manager> {
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

    Ok(lyrics::Manager::new(genius_token, musixmatch_tokens))
}

impl AppState {
    pub async fn init() -> anyhow::Result<&'static Self> {
        log::trace!("Init application");

        let spotify_manager = spotify::Manager::new();
        let lyrics_manager = lyrics_manager()?;

        dotenv::var("CENSOR_BLACKLIST")
            .unwrap_or_default()
            .split(',')
            .for_each(profanity::Manager::add_word);

        dotenv::var("CENSOR_WHITELIST")
            .unwrap_or_default()
            .split(',')
            .for_each(profanity::Manager::remove_word);

        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = db().await?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app_state = Box::new(Self {
            bot,
            spotify_manager,
            lyrics: lyrics_manager,
            db,
        });
        let app_state = &*Box::leak(app_state);

        Ok(app_state)
    }

    pub async fn user_state(&'static self, user_id: &str) -> anyhow::Result<UserState> {
        let spotify = self.spotify_manager.for_user(&self.db, user_id).await?;
        let spotify = RwLock::new(spotify);

        let mut state = UserState {
            app: self,
            spotify,
            spotify_user: None,
            user_id: user_id.to_string(),
        };

        if state.is_spotify_authed().await {
            state.spotify_user = Some(state.spotify.read().await.me().await?);
        }

        Ok(state)
    }
}

pub struct UserState {
    pub app: &'static AppState,
    pub spotify: RwLock<AuthCodeSpotify>,
    pub spotify_user: Option<PrivateUser>,
    pub user_id: String,
}

impl UserState {
    pub async fn is_spotify_authed(&self) -> bool {
        self.spotify
            .read()
            .await
            .token
            .lock()
            .await
            .expect("Failed to acquire lock")
            .is_some()
    }

    pub fn is_spotify_premium(&self) -> bool {
        self.spotify_user
            .as_ref()
            .map(|spotify_user| {
                spotify_user
                    .product
                    .map(|product| product == SubscriptionLevel::Premium)
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    }
}
