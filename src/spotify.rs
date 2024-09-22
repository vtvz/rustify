pub mod errors;
pub mod utils;

use anyhow::{anyhow, Context};
pub use errors::Error;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::http::HttpError;
use rspotify::model::{Context as SpotifyContext, FullTrack, PlayableItem};
use rspotify::{scopes, AuthCodeSpotify, ClientError, ClientResult, Token};
use sea_orm::{DbConn, TransactionTrait};
use strum_macros::Display;

use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::user_service::UserService;

#[derive(Clone, Display)]
pub enum CurrentlyPlayingNoneReason {
    #[strum(serialize = "Current track is on pause")]
    Pause,
    #[strum(serialize = "Nothing is currently playing")]
    Nothing,
    #[strum(serialize = "It's a podcast")]
    Podcast,
    #[strum(serialize = "It's a local file")]
    Local,
}

pub enum CurrentlyPlaying {
    Err(ClientError),
    None(CurrentlyPlayingNoneReason),
    Ok(Box<FullTrack>, Option<SpotifyContext>),
}

impl From<ClientError> for CurrentlyPlaying {
    fn from(err: ClientError) -> Self {
        CurrentlyPlaying::Err(err)
    }
}

impl CurrentlyPlaying {
    pub async fn get(spotify: &AuthCodeSpotify) -> Self {
        let playing = spotify.current_playing(None, None::<&[_]>).await;

        let playing = match playing {
            Ok(playing) => playing,
            Err(err) => return err.into(),
        };

        let (item, context) = match playing {
            Some(playing) => {
                if !playing.is_playing {
                    return Self::None(CurrentlyPlayingNoneReason::Pause);
                }

                (playing.item, playing.context)
            },
            None => return Self::None(CurrentlyPlayingNoneReason::Nothing),
        };

        let item = match item {
            Some(item) => item,
            None => return Self::None(CurrentlyPlayingNoneReason::Nothing),
        };

        let track = match item {
            PlayableItem::Track(item) => item,
            _ => return Self::None(CurrentlyPlayingNoneReason::Podcast),
        };

        match &track.id {
            Some(_) => Self::Ok(Box::new(track), context),
            None => Self::None(CurrentlyPlayingNoneReason::Local),
        }
    }
}

pub struct Manager {
    spotify: AuthCodeSpotify,
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

impl Manager {
    pub fn new() -> Self {
        let config = rspotify::Config {
            token_refreshing: false,
            ..Default::default()
        };

        let creds = rspotify::Credentials::new(
            dotenv::var("SPOTIFY_ID")
                .expect("Env variable SPOTIFY_ID is required")
                .as_ref(),
            dotenv::var("SPOTIFY_SECRET")
                .expect("Env variable SPOTIFY_SECRET is required")
                .as_ref(),
        );

        let oauth = rspotify::OAuth {
            redirect_uri: dotenv::var("SPOTIFY_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8080/callback".into()),
            // TODO Reduce to minimum
            scopes: scopes!(
                "ugc-image-upload",
                "user-read-playback-state",
                "user-modify-playback-state",
                "user-read-currently-playing",
                "user-read-private",
                "user-read-email",
                "user-follow-modify",
                "user-follow-read",
                "user-library-modify",
                "user-library-read",
                "app-remote-control",
                "user-read-playback-position",
                "user-top-read",
                "user-read-recently-played",
                "playlist-modify-private",
                "playlist-read-collaborative",
                "playlist-read-private",
                "playlist-modify-public"
            ),
            ..Default::default()
        };

        let spotify = AuthCodeSpotify::with_config(creds, oauth, config);

        Self { spotify }
    }

    async fn token_refresh(
        db: &DbConn,
        user_id: &str,
        instance: &AuthCodeSpotify,
    ) -> anyhow::Result<()> {
        let should_reauth = instance
            .get_token()
            .lock()
            .await
            .expect("Cannot acquire lock")
            .as_ref()
            .map(Token::is_expired)
            .unwrap_or(false);

        if !should_reauth {
            return Ok(());
        }

        let res = instance.refresh_token().await;

        if !Self::is_token_valid(res).await? {
            {
                let txn = db.begin().await?;

                SpotifyAuthService::remove_token(&txn, user_id).await?;
                UserService::set_status(&txn, user_id, UserStatus::TokenInvalid).await?;

                txn.commit().await?;
            }

            return Err(anyhow!("Token is invalid"));
        };

        let token = instance
            .get_token()
            .lock()
            .await
            .expect("Cannot acquire lock")
            .clone();

        if let Some(token) = token {
            SpotifyAuthService::set_token(db, user_id, token).await?;
        }

        Ok(())
    }

    async fn is_token_valid(mut res: ClientResult<()>) -> anyhow::Result<bool> {
        let response = match res {
            Ok(_) => return Ok(true),
            Err(ClientError::Http(box HttpError::StatusCode(ref mut response))) => response,
            Err(err) => return Err(err.into()),
        };

        let err = Error::from_response(response).await?;

        match err {
            Error::Auth(err) => Ok(err.error != errors::AuthErrorType::InvalidGrant),
            Error::Regular(_) => Ok(true),
        }
    }

    pub async fn for_user(&self, db: &DbConn, user_id: &str) -> anyhow::Result<AuthCodeSpotify> {
        let mut instance = self.spotify.clone();
        instance.token = Default::default();
        let token = SpotifyAuthService::get_token(db, user_id).await?;

        *instance
            .get_token()
            .lock()
            .await
            .expect("Cannot acquire lock") = token;

        Self::token_refresh(db, user_id, &instance).await?;

        Ok(instance)
    }

    #[allow(dead_code)]
    pub async fn get_authorize_url(&self) -> anyhow::Result<String> {
        self.spotify.get_authorize_url(false).context("Get auth")
    }
}
