use anyhow::Context;
use rspotify::AuthCodeSpotify;
use rspotify::clients::OAuthClient;
use rspotify::model::{PrivateUser, SubscriptionLevel};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::entity::prelude::UserLocale;

pub struct UserState {
    pub spotify: RwLock<AuthCodeSpotify>,
    pub user_id: String,

    pub spotify_user: Mutex<Option<Option<PrivateUser>>>,
    pub locale: UserLocale,
}

impl UserState {
    pub fn locale(&self) -> &str {
        self.locale.as_ref()
    }

    pub fn language(&self) -> &str {
        self.locale.language()
    }

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
