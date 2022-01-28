use anyhow::{anyhow, Context};
use genius_rs::Genius;
use rspotify::AuthCodeSpotify;
use sea_orm::{Database, DatabaseConnection, DbConn};
use sqlx::migrate::MigrateDatabase;
use teloxide::Bot;
use tokio::sync::RwLock;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::{profanity, spotify};

pub struct AppState {
    pub spotify_manager: spotify::Manager,
    pub genius: Genius,
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

fn logger() -> anyhow::Result<()> {
    let tracing_init = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(true)
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .finish()
        .with(
            Targets::new()
                .with_target(
                    &env!("CARGO_PKG_NAME").replace('-', "_"),
                    tracing::Level::TRACE,
                )
                .with_target("teloxide", tracing::Level::INFO)
                .with_default(tracing::Level::WARN),
        )
        .try_init();

    match tracing_init {
        Ok(_) => log::info!("tracing_subscriber::fmt::try_init success"),
        Err(err) => log::error!(
            "tracing_subscriber::fmt::try_init error: {:?}",
            anyhow!(err)
        ),
    }

    Ok(())
}

impl AppState {
    pub async fn init() -> anyhow::Result<&'static Self> {
        logger()?;

        log::trace!("Init application");

        let spotify_manager = spotify::Manager::new();
        let genius = genius().await?;

        dotenv::var("CENSOR_BLACKLIST")
            .unwrap_or_default()
            .split(',')
            .for_each(profanity::Manager::add_word);

        let bot = Bot::new(
            dotenv::var("TELEGRAM_BOT_TOKEN").context("Need TELEGRAM_BOT_TOKEN variable")?,
        );

        let db = db().await?;

        // Make state global static variable to prevent hassle with Arc and cloning this mess
        let app_state = Box::new(Self {
            bot,
            spotify_manager,
            genius,
            db,
        });
        let app_state = &*Box::leak(app_state);

        Ok(app_state)
    }

    pub async fn user_state(&'static self, user_id: &str) -> anyhow::Result<UserState> {
        Ok(UserState {
            app: self,
            spotify: RwLock::new(self.spotify_manager.for_user(&self.db, user_id).await?),
            user_id: user_id.to_string(),
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
