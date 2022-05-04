use anyhow::anyhow;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::http::HttpError;
use rspotify::model::{Context as SpotifyContext, FullTrack, Id, PlayableItem};
use rspotify::{scopes, AuthCodeSpotify, ClientError, ClientResult, Token};
use sea_orm::{DbConn, TransactionTrait};
use teloxide::utils::markdown;

use crate::entity::prelude::*;
use crate::errors::{Context, GenericResult};
use crate::spotify_auth_service::SpotifyAuthService;
use crate::user_service::UserService;

pub enum CurrentlyPlaying {
    Err(ClientError),
    None(&'static str),
    Ok(Box<FullTrack>, Option<SpotifyContext>),
}

impl From<ClientError> for CurrentlyPlaying {
    fn from(err: ClientError) -> Self {
        CurrentlyPlaying::Err(err)
    }
}

pub async fn currently_playing(spotify: &AuthCodeSpotify) -> CurrentlyPlaying {
    let playing = spotify.current_playing(None, None::<&[_]>).await;

    let playing = match playing {
        Ok(playing) => playing,
        Err(err) => return err.into(),
    };

    let (item, context) = match playing {
        Some(playing) => {
            if !playing.is_playing {
                return CurrentlyPlaying::None("Current track is on pause");
            }

            (playing.item, playing.context)
        },
        None => return CurrentlyPlaying::None("Nothing is currently playing"),
    };

    let item = match item {
        Some(item) => item,
        None => return CurrentlyPlaying::None("Nothing is currently playing"),
    };

    let track = match item {
        PlayableItem::Track(item) => item,
        _ => return CurrentlyPlaying::None("It's a podcast"),
    };

    match &track.id {
        Some(_) => CurrentlyPlaying::Ok(Box::new(track), context),
        None => CurrentlyPlaying::None("It's a local file"),
    }
}

pub fn artist_names(track: &FullTrack) -> Vec<String> {
    track.artists.iter().map(|art| art.name.clone()).collect()
}

pub fn get_track_id(track: &FullTrack) -> String {
    track
        .id
        .as_ref()
        .map(|track_id| track_id.id().to_owned())
        .unwrap_or_default()
}

pub fn create_track_tg_link(track: &FullTrack) -> String {
    format!(
        r#"[{}]({})"#,
        markdown::escape(create_track_name(track).as_str()),
        track
            .external_urls
            .get("spotify")
            .cloned()
            .unwrap_or_else(|| "https://vtvz.me/".into())
    )
}

pub fn create_track_name(track: &FullTrack) -> String {
    let artists = artist_names(track).join(", ");

    format!(r#"{} — {}"#, &artists, &track.name)
}

pub struct Manager {
    spotify: AuthCodeSpotify,
}

impl Manager {
    pub fn new() -> Self {
        let config = rspotify::Config::default();

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
    ) -> GenericResult<()> {
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

            return Err(anyhow!("Token is invalid").into());
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

    async fn is_token_valid(res: ClientResult<()>) -> GenericResult<bool> {
        let mut response = match res {
            Ok(_) => return Ok(true),
            Err(ClientError::Http(box HttpError::StatusCode(response))) => response,
            Err(err) => return Err(err.into()),
        };

        let body = {
            let mut bytes = vec![];
            while let Some(chunk) = response.chunk().await? {
                bytes.extend(chunk);
            }
            String::from_utf8(bytes)?
        };

        let json: serde_json::Value = serde_json::from_str(&body)?;

        if json["error"].as_str() == Some("invalid_grant") {
            return Ok(false);
        }

        Err(ClientError::Http(Box::new(HttpError::StatusCode(response))))?
    }

    pub async fn for_user(&self, db: &DbConn, user_id: &str) -> GenericResult<AuthCodeSpotify> {
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
    pub async fn get_authorize_url(&self) -> GenericResult<String> {
        self.spotify.get_authorize_url(false).context("Get auth")
    }
}
