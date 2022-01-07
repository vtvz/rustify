use crate::spotify;
use anyhow::Context;
use censor::{Censor, Standard};
use genius_rs::Genius;
use rspotify::AuthCodeSpotify;
use sea_orm::{Database, DatabaseConnection, DbConn};
use sqlx::migrate::MigrateDatabase;
use std::sync::Arc;
use teloxide::Bot;

pub struct AppState {
    pub spotify_manager: spotify::Manager,
    pub genius: Arc<Genius>,
    pub censor: Censor,
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

pub struct UserState<'a> {
    pub app: &'a AppState,
    pub spotify: AuthCodeSpotify,
}

impl AppState {
    pub async fn init() -> anyhow::Result<&'static Self> {
        let spotify_manager = spotify::Manager::new();
        let genius = Arc::new(genius().await?);
        let censor =
            censor::Standard + censor::Sex + Censor::custom(vec!["christmas", "xmas", "halloween"]);

        teloxide::enable_logging!();
        log::info!("Starting rustify bot...");
        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = db().await?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app_state = Self {
            bot: bot.clone(),
            spotify_manager,
            genius,
            censor,
            db,
        };
        let app_state = Box::new(app_state);
        let app_state = &*Box::leak(app_state);
        Ok(app_state)
    }

    pub async fn user_state(&'static self, user_id: String) -> UserState<'static> {
        UserState {
            app: self,
            spotify: self.spotify_manager.for_user(user_id).await,
        }
    }
}
