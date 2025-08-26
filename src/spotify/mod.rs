pub mod errors;

use std::borrow::Cow;

use anyhow::{Context, anyhow};
pub use errors::SpotifyError;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::http::HttpError;
use rspotify::model::{
    ArtistId,
    Context as SpotifyContext,
    FullPlaylist,
    FullTrack,
    Id,
    PlayableItem,
    PlaylistId,
    SimplifiedPlaylist,
    TrackId,
};
use rspotify::{AuthCodeSpotify, ClientError, ClientResult, Token, scopes};
use sea_orm::{DbConn, TransactionTrait};
use teloxide::utils::html;

use crate::entity::prelude::*;
use crate::spotify_auth_service::SpotifyAuthService;
use crate::user_service::UserService;

pub struct ShortPlaylist {
    id: PlaylistId<'static>,
    url: String,
}

impl ShortPlaylist {
    pub fn id(&self) -> &PlaylistId<'static> {
        &self.id
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl From<FullPlaylist> for ShortPlaylist {
    fn from(value: FullPlaylist) -> Self {
        Self {
            id: value.id,
            url: value.external_urls.get("spotify").cloned().unwrap_or(
                "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT?si=23c50743cbd5462b".into(),
            ),
        }
    }
}

impl From<SimplifiedPlaylist> for ShortPlaylist {
    fn from(value: SimplifiedPlaylist) -> Self {
        Self {
            id: value.id,
            url: value.external_urls.get("spotify").cloned().unwrap_or(
                "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT?si=23c50743cbd5462b".into(),
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ShortTrack {
    id: TrackId<'static>,
    name: String,
    url: String,
    duration_secs: i64,
    artist_names: Vec<String>,
    artist_ids: Vec<ArtistId<'static>>,
    album_name: String,
    album_url: String,
}

impl ShortTrack {
    pub fn new(full_track: FullTrack) -> Self {
        Self {
            id: full_track
                .id
                .unwrap_or(TrackId::from_id("4PTG3Z6ehGkBFwjybzWkR8").expect("Valid ID")),
            name: full_track.name,

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

            url: full_track
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

    pub fn id(&self) -> &str {
        self.id.id()
    }

    pub fn raw_id(&self) -> &TrackId<'_> {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn name_with_artists(&self) -> String {
        let artists = self.artist_names().join(", ");

        format!(r#"{} — {}"#, artists, self.name())
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
        self.artist_names()
            .first()
            .copied()
            .unwrap_or("Rick Astley")
    }

    pub fn album_name(&self) -> &str {
        &self.album_name
    }

    pub fn album_url(&self) -> &str {
        &self.album_url
    }

    pub fn track_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = html::escape(self.name_with_artists().as_str()),
            link = self.url()
        )
    }

    pub fn album_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = self.album_name(),
            link = self.album_url()
        )
    }
}

impl From<FullTrack> for ShortTrack {
    fn from(value: FullTrack) -> Self {
        ShortTrack::new(value)
    }
}

#[derive(Clone)]
pub enum CurrentlyPlayingNoneReason {
    Pause,
    Nothing,
    Podcast,
    Local,
}

impl CurrentlyPlayingNoneReason {
    pub fn localize(&self, locale: &str) -> Cow<'_, str> {
        match self {
            Self::Pause => {
                t!("currently-playing-none-reason.pause", locale = locale)
            },
            Self::Nothing => {
                t!("currently-playing-none-reason.nothing", locale = locale)
            },
            Self::Podcast => {
                t!("currently-playing-none-reason.podcast", locale = locale)
            },
            Self::Local => {
                t!("currently-playing-none-reason.local", locale = locale)
            },
        }
    }
}

pub enum CurrentlyPlaying {
    Err(ClientError),
    None(CurrentlyPlayingNoneReason),
    Ok(Box<ShortTrack>, Option<SpotifyContext>),
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
            Some(_) => Self::Ok(Box::new(track.into()), context),
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

        let err = SpotifyError::from_response(response).await?;

        match err {
            SpotifyError::Auth(err) => Ok(err.error != errors::AuthErrorType::InvalidGrant),
            SpotifyError::Regular(_) => Ok(true),
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
