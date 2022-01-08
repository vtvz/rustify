use anyhow::Context;
use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{FullTrack, Id, PlayableItem};
use rspotify::{scopes, AuthCodeSpotify};
use sea_orm::DbConn;
use teloxide::utils::markdown::escape;

use crate::spotify_auth_service::SpotifyAuthService;

pub enum CurrentlyPlaying {
    Err(anyhow::Error),
    None(String),
    Ok(Box<FullTrack>),
}

impl<E> From<E> for CurrentlyPlaying
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        CurrentlyPlaying::Err(err.into())
    }
}

pub async fn currently_playing(spotify: &AuthCodeSpotify) -> CurrentlyPlaying {
    let playing = spotify.current_playing(None, None::<&[_]>).await;

    let playing = match playing {
        Ok(playing) => playing,
        Err(err) => return err.into(),
    };

    let playing = match playing {
        Some(playing) => {
            if !playing.is_playing {
                return CurrentlyPlaying::None("Current track is on pause".into());
            }

            playing.item
        }
        None => return CurrentlyPlaying::None("Nothing is currently playing".into()),
    };

    let item = match playing {
        Some(item) => item,
        None => return CurrentlyPlaying::None("Nothing is currently playing".into()),
    };

    let track = match item {
        PlayableItem::Track(item) => item,
        _ => return CurrentlyPlaying::None("It's a podcast".into()),
    };

    match &track.id {
        Some(_) => CurrentlyPlaying::Ok(Box::new(track)),
        None => CurrentlyPlaying::None("It's a local file".into()),
    }
}

pub fn artist_names(track: &FullTrack) -> Vec<String> {
    track.artists.iter().map(|art| art.name.clone()).collect()
}

pub fn get_track_id(track: &FullTrack) -> String {
    track
        .id
        .clone()
        .expect("Should be validated beforehand")
        .id()
        .to_owned()
}

pub fn create_track_name(track: &FullTrack) -> String {
    let artists = artist_names(track).join(", ");

    format!(
        r#"[{} — {}]({})"#,
        escape(&artists),
        escape(&track.name),
        track
            .external_urls
            .get("spotify")
            .cloned()
            .unwrap_or_else(|| "https://vtvz.me/".into())
    )
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
            redirect_uri: "http://localhost:8080/callback".into(),
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

        let spotify = rspotify::AuthCodeSpotify::with_config(creds, oauth, config);

        Self { spotify }
    }

    pub async fn for_user(&self, db: &DbConn, user_id: String) -> anyhow::Result<AuthCodeSpotify> {
        let instance = self.spotify.clone();
        let token = SpotifyAuthService::get_token(db, user_id).await?;

        *instance.token.lock().await.expect("Cannot acquire lock") = token;

        instance.refresh_token().await?;

        Ok(instance)
    }

    #[allow(dead_code)]
    pub async fn get_authorize_url(&self) -> anyhow::Result<String> {
        self.spotify.get_authorize_url(false).context("Get auth")
    }
}
