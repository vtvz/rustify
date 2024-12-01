pub mod errors;
pub mod utils;

use anyhow::{Context, anyhow};
pub use errors::Error;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::http::HttpError;
use rspotify::model::{ArtistId, Context as SpotifyContext, FullTrack, Id, PlayableItem, TrackId};
use rspotify::{AuthCodeSpotify, ClientError, ClientResult, Token, scopes};
use sea_orm::{DbConn, TransactionTrait};
use strum_macros::Display;
use teloxide::utils::html;

use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::user_service::UserService;

#[derive(Serialize, Deserialize)]
pub struct ShortTrack {
    track_id: String,
    track_name: String,
    duration_secs: i64,
    artist_names: Vec<String>,
    artist_ids: Vec<ArtistId<'static>>,
    track_url: String,
    album_name: String,
    album_url: String,
}

impl ShortTrack {
    pub fn new(full_track: FullTrack) -> Self {
        Self {
            track_id: full_track
                .id
                .as_ref()
                .map(|track_id| track_id.id().into())
                .unwrap_or_default(),
            track_name: full_track.name,
            duration_secs: full_track.duration.num_seconds(),

            artist_names: full_track
                .artists
                .iter()
                .map(|art| art.name.clone())
                .collect(),

            artist_ids: full_track
                .artists
                .iter()
                .filter_map(|artist| artist.id.clone())
                .collect(),

            track_url: full_track
                .external_urls
                .get("spotify")
                .cloned()
                .unwrap_or("https://vtvz.me/".into()),

            album_url: full_track
                .album
                .external_urls
                .get("spotify")
                .cloned()
                .unwrap_or("https://vtvz.me/".into()),

            album_name: full_track.album.name,
        }
    }

    pub fn track_id(&self) -> &str {
        &self.track_id
    }

    pub fn track_raw_id(&self) -> anyhow::Result<TrackId<'_>> {
        let id = TrackId::from_id_or_uri(self.track_id())?;
        Ok(id)
    }

    pub fn track_full_name(&self) -> String {
        let artists = self.artist_names().join(", ");

        format!(r#"{} — {}"#, artists, self.track_name())
    }

    pub fn track_name(&self) -> &str {
        &self.track_name
    }

    pub fn duration_secs(&self) -> i64 {
        self.duration_secs
    }

    pub fn artist_names(&self) -> Vec<&str> {
        self.artist_names.iter().map(|item| item.as_str()).collect()
    }

    pub fn artist_ids(&self) -> Vec<&str> {
        self.artist_ids.iter().map(|artist| artist.id()).collect()
    }

    pub fn artist_raw_ids(&self) -> &[ArtistId<'_>] {
        &self.artist_ids
    }

    pub fn first_artist_name(&self) -> &str {
        self.artist_names().first().copied().unwrap_or_default()
    }

    pub fn track_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = html::escape(self.track_full_name().as_str()),
            link = self.track_url
        )
    }

    pub fn album_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = self.album_name(),
            link = self.album_url()
        )
    }

    pub fn album_name(&self) -> &str {
        &self.album_name
    }

    pub fn album_url(&self) -> &str {
        &self.album_url
    }
}

impl From<FullTrack> for ShortTrack {
    fn from(value: FullTrack) -> Self {
        ShortTrack::new(value)
    }
}

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
