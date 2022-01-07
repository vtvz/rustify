use rspotify::clients::OAuthClient;
use rspotify::model::{FullTrack, Id, PlayableItem};
use rspotify::{scopes, AuthCodeSpotify, Token};
use std::fmt::format;

pub enum CurrentlyPlaying {
    Error(anyhow::Error),
    None(String),
    Ok(Box<FullTrack>),
}

impl<E> From<E> for CurrentlyPlaying
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        CurrentlyPlaying::Error(error.into())
    }
}

pub async fn currently_playing(spotify: &AuthCodeSpotify) -> CurrentlyPlaying {
    let playing = spotify.current_playing(None, None::<&[_]>).await;

    let playing = match playing {
        Ok(playing) => playing,
        Err(error) => return error.into(),
    };

    let playing = match playing {
        Some(playing) => {
            if !playing.is_playing {
                // return CurrentlyPlaying::None("Current track is on pause".into());
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
    track.id.clone().unwrap().id().to_string()
}

pub fn create_track_name(track: &FullTrack) -> String {
    let artists = artist_names(track).join(", ");

    format!(
        r#"[{} — {}]({})"#,
        artists,
        track.name,
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
        let config = rspotify::Config {
            token_refreshing: true,
            ..Default::default()
        };

        let creds = rspotify::Credentials::new(
            dotenv::var("SPOTIFY_ID").unwrap().as_ref(),
            dotenv::var("SPOTIFY_SECRET").unwrap().as_ref(),
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

        let spotify = rspotify::AuthCodeSpotify::with_config(creds.clone(), oauth, config.clone());

        Self { spotify }
    }

    pub async fn for_user(&self, _user_id: String) -> AuthCodeSpotify {
        let instance = self.spotify.clone();
        *instance.token.lock().await.unwrap() = Some(Token {
            access_token: dotenv::var("SPOTIFY_ACCESS_TOKEN").unwrap(),
            refresh_token: Some(dotenv::var("SPOTIFY_REFRESH_TOKEN").unwrap()),
            ..Default::default()
        });

        instance
    }

    pub async fn get_authorize_url(&self) -> String {
        self.spotify.get_authorize_url(false).unwrap()
    }
}
