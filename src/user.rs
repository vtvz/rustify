use anyhow::Context as _;
use rspotify::AuthCodeSpotify;
use rspotify::clients::OAuthClient as _;
use rspotify::model::{PrivateUser, SubscriptionLevel};
use teloxide::types::ChatId;
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::entity::prelude::UserModel;
use crate::spotify::SpotifyWrapper;

pub struct UserState {
    spotify: RwLock<AuthCodeSpotify>,
    #[allow(clippy::option_option)]
    spotify_user: Mutex<Option<Option<PrivateUser>>>,
    user: UserModel,
    newly_created: bool,
}

pub type SpotifyWrapperType<'a> = SpotifyWrapper<RwLockReadGuard<'a, AuthCodeSpotify>>;

impl UserState {
    #[must_use]
    pub fn new(user: UserModel, newly_created: bool, spotify: AuthCodeSpotify) -> Self {
        Self {
            spotify: RwLock::new(spotify),
            spotify_user: Mutex::default(),
            user,
            newly_created,
        }
    }

    pub fn user(&self) -> &UserModel {
        &self.user
    }

    pub fn newly_created(&self) -> bool {
        self.newly_created
    }

    pub fn locale(&self) -> &str {
        self.user().locale.as_ref()
    }

    pub fn language(&self) -> &str {
        self.user().locale.language()
    }

    pub async fn spotify(&self) -> SpotifyWrapperType<'_> {
        let lock = self.spotify.read().await;

        SpotifyWrapper::new(lock)
    }

    pub async fn spotify_write(&self) -> RwLockWriteGuard<'_, AuthCodeSpotify> {
        self.spotify.write().await
    }

    pub fn user_id(&self) -> &str {
        &self.user.id
    }

    pub fn chat_id(&self) -> anyhow::Result<ChatId> {
        Ok(ChatId(self.user_id().parse()?))
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
        let res = self.spotify_user().await?.is_some_and(|spotify_user| {
            spotify_user
                .product
                .is_some_and(|product| product == SubscriptionLevel::Premium)
        });

        Ok(res)
    }
}
