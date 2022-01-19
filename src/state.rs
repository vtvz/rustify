use std::sync::Arc;

use anyhow::Context;
use genius_rs::Genius;
use rspotify::AuthCodeSpotify;
use rustrict::Type;
use sea_orm::{Database, DatabaseConnection, DbConn};
use sqlx::migrate::MigrateDatabase;
use teloxide::Bot;
use tokio::sync::RwLock;

use crate::spotify;

pub struct AppState {
    pub spotify_manager: spotify::Manager,
    pub genius: Arc<Genius>,
    pub bot: Bot,
    pub db: DatabaseConnection,
}

async fn db() -> anyhow::Result<DbConn> {
    let database_url = dotenv::var("DATABASE_URL").context("Needs DATABASE_URL")?;

    sqlx::Sqlite::create_database(&database_url)
        .await
        .context("Create database")?;

    let pool = sqlx::SqlitePool::connect(&database_url)
        .await
        .context("Cannot connect DB")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Cannot migrate")?;

    pool.close().await;

    Ok(Database::connect(database_url)
        .await
        .context("Cannot connect DB")?)
}

async fn genius() -> anyhow::Result<Genius> {
    Ok(Genius::new(
        dotenv::var("GENIUS_ACCESS_TOKEN").context("Needs GENIUS_ACCESS_TOKEN")?,
    ))
}

impl AppState {
    pub async fn init() -> anyhow::Result<&'static Self> {
        let spotify_manager = spotify::Manager::new();
        let genius = Arc::new(genius().await?);

        dotenv::var("CENSOR_BLACKLIST")
            .unwrap_or_default()
            .split(',')
            .for_each(|word| unsafe {
                rustrict::add_word(word, Type::MODERATE);
            });

        teloxide::enable_logging!();
        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = db().await?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app_state = Self {
            bot: bot.clone(),
            spotify_manager,
            genius,
            db,
        };
        let app_state = Box::new(app_state);
        let app_state = &*Box::leak(app_state);
        Ok(app_state)
    }

    pub async fn user_state(&'static self, user_id: String) -> anyhow::Result<UserState> {
        Ok(UserState {
            app: self,
            spotify: RwLock::new(
                self.spotify_manager
                    .for_user(&self.db, user_id.clone())
                    .await?,
            ),
            user_id,
        })
    }
}

pub struct UserState {
    pub app: &'static AppState,
    pub spotify: RwLock<AuthCodeSpotify>,
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
}
