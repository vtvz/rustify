use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::spotify::ShortTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongLinkResponse {
    pub entity_unique_id: String,
    pub user_country: String,
    pub page_url: String,
    pub links_by_platform: HashMap<SongLinkPlatform, SongLinkPlatformLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SongLinkPlatformLink {
    pub entity_unique_id: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_app_uri_mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_app_uri_desktop: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum SongLinkPlatform {
    Spotify,
    Itunes,
    AppleMusic,
    Youtube,
    YoutubeMusic,
    Google,
    GoogleStore,
    Pandora,
    Deezer,
    Tidal,
    AmazonStore,
    AmazonMusic,
    Soundcloud,
    Napster,
    Yandex,
    Spinrilla,
    Audius,
    Audiomack,
    Anghami,
    Boomplay,
    Bandcamp,
}

impl Display for SongLinkPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::AmazonMusic => "Amazon Music",
            Self::AmazonStore => "Amazon Store",
            Self::Anghami => "Anghami",
            Self::AppleMusic => "Apple Music",
            Self::Audiomack => "Audiomack",
            Self::Audius => "Audius",
            Self::Bandcamp => "Bandcamp",
            Self::Boomplay => "Boomplay",
            Self::Deezer => "Deezer",
            Self::Google => "Google",
            Self::GoogleStore => "Google Store",
            Self::Itunes => "iTunes",
            Self::Napster => "Napster",
            Self::Pandora => "Pandora",
            Self::Soundcloud => "SoundCloud",
            Self::Spinrilla => "Spinrilla",
            Self::Spotify => "Spotify",
            Self::Tidal => "Tidal",
            Self::Yandex => "Yandex",
            Self::Youtube => "YouTube",
            Self::YoutubeMusic => "YouTube Music",
        };

        f.write_str(name)
    }
}

pub struct SongLinkService {
    client: reqwest::Client,
}

impl SongLinkService {
    #[must_use]
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn get(&self, track: &ShortTrack) -> anyhow::Result<SongLinkResponse> {
        // https://odesli.co/
        let mut url = Url::parse("https://api.song.link/v1-alpha.1/links").expect("Parsable");

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("url", track.url());
            pairs.append_pair("songIfSingle", "true");
        }

        let res_text = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let res = serde_json::from_str::<SongLinkResponse>(&res_text)
            .with_context(|| format!("Failed parsing json response:\n{res_text}"))?;

        Ok(res)
    }
}
