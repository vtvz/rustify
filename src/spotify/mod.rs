pub mod auth;
pub mod errors;

use std::borrow::Cow;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{Context, anyhow};
use auth::SpotifyAuthService;
use chrono::Duration;
pub use errors::SpotifyError;
use redis::AsyncCommands;
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
use crate::services::UserService;
use crate::user::UserState;

pub struct ShortPlaylist {
    id: PlaylistId<'static>,
    url: String,
}

impl ShortPlaylist {
    #[must_use]
    pub fn id(&self) -> &PlaylistId<'static> {
        &self.id
    }

    #[must_use]
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
    #[must_use]
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

    #[must_use]
    pub fn id(&self) -> &str {
        self.id.id()
    }

    #[must_use]
    pub fn raw_id(&self) -> &TrackId<'_> {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn name_with_artists(&self) -> String {
        let artists = self.artist_names().join(", ");

        format!("{} — {}", artists, self.name())
    }

    #[must_use]
    pub fn duration_secs(&self) -> i64 {
        self.duration_secs
    }

    #[must_use]
    pub fn artist_names(&self) -> Vec<&str> {
        self.artist_names.iter().map(String::as_str).collect()
    }

    #[must_use]
    pub fn artist_ids(&self) -> Vec<&str> {
        self.artist_ids.iter().map(Id::id).collect()
    }

    #[must_use]
    pub fn artist_raw_ids(&self) -> &[ArtistId<'_>] {
        &self.artist_ids
    }

    #[must_use]
    pub fn first_artist_name(&self) -> &str {
        self.artist_names()
            .first()
            .copied()
            .unwrap_or("Rick Astley")
    }

    #[must_use]
    pub fn album_name(&self) -> &str {
        &self.album_name
    }

    #[must_use]
    pub fn album_url(&self) -> &str {
        &self.album_url
    }

    #[must_use]
    pub fn track_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = html::escape(self.name_with_artists().as_str()),
            link = self.url()
        )
    }

    #[must_use]
    pub fn album_tg_link(&self) -> String {
        format!(
            r#"<a href="{link}">{name}</a>"#,
            name = html::escape(self.album_name()),
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
    #[must_use]
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

pub struct Manager {
    spotify: AuthCodeSpotify,
}

impl Manager {
    #[must_use]
    pub fn new(
        spotify_id: &str,
        spotify_secret: &str,
        spotify_redirect_uri: Option<String>,
    ) -> Self {
        let config = rspotify::Config {
            token_refreshing: false,
            ..Default::default()
        };

        let creds = rspotify::Credentials::new(spotify_id, spotify_secret);

        let oauth = rspotify::OAuth {
            redirect_uri: spotify_redirect_uri
                .unwrap_or_else(|| "http://localhost:8080/callback".into()),
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
            .is_some_and(Token::is_expired);

        if !should_reauth {
            return Ok(());
        }

        let res = instance.refresh_token().await;

        if !Self::is_token_valid(res).await? {
            {
                let txn = db.begin().await?;

                UserService::set_status(&txn, user_id, UserStatus::SpotifyTokenInvalid).await?;

                txn.commit().await?;
            }

            return Err(anyhow!("Token is invalid"));
        }

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
            Ok(()) => return Ok(true),
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
        instance.token = Arc::default();
        let token = SpotifyAuthService::get_token(db, user_id).await?;

        *instance
            .get_token()
            .lock()
            .await
            .expect("Cannot acquire lock") = token;

        Self::token_refresh(db, user_id, &instance).await?;

        Ok(instance)
    }

    pub async fn get_authorize_url(&self, state: &UserState) -> anyhow::Result<String> {
        let mut spotify = state.spotify().await.clone();

        spotify.oauth.state = state.user().spotify_state.to_string();

        spotify.get_authorize_url(false).context("Get auth")
    }
}

pub struct SpotifyWrapper<S> {
    spotify: S,
}

impl<S> SpotifyWrapper<S> {
    pub fn new(spotify: S) -> Self {
        Self { spotify }
    }
}

impl<S: Deref<Target = AuthCodeSpotify>> SpotifyWrapper<S> {
    pub async fn short_track_cached(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track_id: TrackId<'_>,
    ) -> anyhow::Result<ShortTrack> {
        let key = format!("rustify:track_data:{}", track_id.id());
        let ttl = Duration::hours(1);

        let data: Option<String> = redis_conn.get(&key).await?;

        if let Some(data) = data {
            if let Ok(track) = serde_json::from_str(&data) {
                return Ok(track);
            }
        }

        let track: ShortTrack = self.spotify.track(track_id, None).await?.into();

        let _: () = redis_conn
            .set_ex(key, serde_json::to_string(&track)?, ttl.num_seconds() as _)
            .await?;

        Ok(track)
    }

    pub async fn current_playing_wrapped(&self) -> CurrentlyPlaying {
        let playing = self.current_playing(None, None::<&[_]>).await;

        let playing = match playing {
            Ok(playing) => playing,
            Err(err) => return err.into(),
        };

        let (item, context) = match playing {
            Some(playing) => {
                if !playing.is_playing {
                    return CurrentlyPlaying::None(CurrentlyPlayingNoneReason::Pause);
                }

                (playing.item, playing.context)
            },
            None => return CurrentlyPlaying::None(CurrentlyPlayingNoneReason::Nothing),
        };

        let Some(item) = item else {
            return CurrentlyPlaying::None(CurrentlyPlayingNoneReason::Nothing);
        };

        let PlayableItem::Track(track) = item else {
            return CurrentlyPlaying::None(CurrentlyPlayingNoneReason::Podcast);
        };

        match &track.id {
            Some(_) => CurrentlyPlaying::Ok(Box::new(track.into()), context),
            None => CurrentlyPlaying::None(CurrentlyPlayingNoneReason::Local),
        }
    }
}

impl<T: Deref<Target = AuthCodeSpotify>> Deref for SpotifyWrapper<T> {
    type Target = AuthCodeSpotify;

    fn deref(&self) -> &AuthCodeSpotify {
        &self.spotify
    }
}
