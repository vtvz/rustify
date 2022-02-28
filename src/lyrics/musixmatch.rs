use itertools::Itertools;
use rand::prelude::*;
use rand::seq::SliceRandom;
use reqwest::Client;
use rspotify::model::FullTrack;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{from_value, Value};
use tokio::sync::Mutex;

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(u8::deserialize(deserializer)? != 0)
}

fn lines_from_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(String::deserialize(deserializer)?
        .lines()
        .map(str::to_owned)
        .collect())
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Root {
    #[serde(rename = "track_id")]
    pub track_id: i64,
    #[serde(rename = "track_mbid")]
    pub track_mbid: String,
    #[serde(rename = "track_isrc")]
    pub track_isrc: String,
    #[serde(rename = "commontrack_isrcs")]
    pub commontrack_isrcs: Vec<Vec<String>>,
    #[serde(rename = "track_spotify_id")]
    pub track_spotify_id: String,
    #[serde(rename = "commontrack_spotify_ids")]
    pub commontrack_spotify_ids: Vec<String>,
    #[serde(rename = "track_soundcloud_id")]
    pub track_soundcloud_id: i64,
    #[serde(rename = "track_xboxmusic_id")]
    pub track_xboxmusic_id: String,
    #[serde(rename = "track_name")]
    pub track_name: String,
    #[serde(rename = "track_name_translation_list")]
    pub track_name_translation_list: Vec<Value>,
    #[serde(rename = "track_rating")]
    pub track_rating: i64,
    #[serde(rename = "track_length")]
    pub track_length: i64,
    #[serde(rename = "commontrack_id")]
    pub commontrack_id: i64,
    pub instrumental: i64,
    pub explicit: i64,
    #[serde(rename = "has_lyrics")]
    pub has_lyrics: i64,
    #[serde(rename = "has_lyrics_crowd")]
    pub has_lyrics_crowd: i64,
    #[serde(rename = "has_subtitles")]
    pub has_subtitles: i64,
    #[serde(rename = "has_richsync")]
    pub has_richsync: i64,
    #[serde(rename = "has_track_structure")]
    pub has_track_structure: i64,
    #[serde(rename = "num_favourite")]
    pub num_favourite: i64,
    #[serde(rename = "lyrics_id")]
    pub lyrics_id: i64,
    #[serde(rename = "subtitle_id")]
    pub subtitle_id: i64,
    #[serde(rename = "album_id")]
    pub album_id: i64,
    #[serde(rename = "album_name")]
    pub album_name: String,
    #[serde(rename = "artist_id")]
    pub artist_id: i64,
    #[serde(rename = "artist_mbid")]
    pub artist_mbid: String,
    #[serde(rename = "artist_name")]
    pub artist_name: String,
    #[serde(rename = "album_coverart_100x100")]
    pub album_coverart_100x100: String,
    #[serde(rename = "album_coverart_350x350")]
    pub album_coverart_350x350: String,
    #[serde(rename = "album_coverart_500x500")]
    pub album_coverart_500x500: String,
    #[serde(rename = "album_coverart_800x800")]
    pub album_coverart_800x800: String,
    #[serde(rename = "track_share_url")]
    pub track_share_url: String,
    #[serde(rename = "track_edit_url")]
    pub track_edit_url: String,
    #[serde(rename = "commontrack_vanity_id")]
    pub commontrack_vanity_id: String,
    pub restricted: i64,
    #[serde(rename = "first_release_date")]
    pub first_release_date: String,
    #[serde(rename = "updated_time")]
    pub updated_time: String,
    #[serde(rename = "primary_genres")]
    pub primary_genres: PrimaryGenres,
    #[serde(rename = "secondary_genres")]
    pub secondary_genres: SecondaryGenres,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PrimaryGenres {
    #[serde(rename = "music_genre_list")]
    pub music_genre_list: Vec<MusicGenreList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MusicGenreList {
    #[serde(rename = "music_genre")]
    pub music_genre: MusicGenre,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MusicGenre {
    #[serde(rename = "music_genre_id")]
    pub music_genre_id: i64,
    #[serde(rename = "music_genre_parent_id")]
    pub music_genre_parent_id: i64,
    #[serde(rename = "music_genre_name")]
    pub music_genre_name: String,
    #[serde(rename = "music_genre_name_extended")]
    pub music_genre_name_extended: String,
    #[serde(rename = "music_genre_vanity")]
    pub music_genre_vanity: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SecondaryGenres {
    #[serde(rename = "music_genre_list")]
    pub music_genre_list: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Lyrics {
    #[serde(rename = "lyrics_id")]
    pub id: u64,
    #[serde(deserialize_with = "bool_from_int")]
    pub verified: bool,
    #[serde(deserialize_with = "bool_from_int")]
    pub restricted: bool,
    #[serde(deserialize_with = "bool_from_int")]
    pub instrumental: bool,
    #[serde(deserialize_with = "bool_from_int")]
    pub explicit: bool,
    #[serde(deserialize_with = "lines_from_string", rename = "lyrics_body")]
    pub lyrics: Vec<String>,
    #[serde(rename = "lyrics_language")]
    pub language: String,
    #[serde(rename = "lyrics_language_description")]
    pub language_description: String,
    #[serde(rename = "backlink_url")]
    pub backlink_url: String,
}

impl super::SearchResult for Lyrics {
    fn lyrics(&self) -> &Vec<String> {
        &self.lyrics
    }

    fn tg_link(&self, text: &str) -> String {
        text.to_string()
    }
}

pub struct Musixmatch {
    reqwest: Client,
    tokens: Vec<String>,
    rnd: Mutex<StdRng>,
}

impl Musixmatch {
    pub fn new(tokens: Vec<String>) -> Self {
        Self {
            reqwest: Client::new(),
            tokens,
            rnd: Mutex::new(StdRng::from_entropy()),
        }
    }

    pub async fn search_for_track(&self, track: &FullTrack) -> anyhow::Result<Option<Vec<String>>> {
        let mut url =
            reqwest::Url::parse("https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get")?;

        // Static
        url.query_pairs_mut().extend_pairs(&[
            ("format", "json"),
            ("namespace", "lyrics_synched"),
            ("subtitle_format", "mxm"),
            ("app_id", "web-desktop-app-v1.0"),
        ]);

        let artists = track
            .artists
            .iter()
            .map(|artist| artist.name.as_str())
            .join(",");

        let usertoken = self
            .tokens
            .choose(&mut *self.rnd.lock().await)
            .cloned()
            .expect("Should exist");

        // Dynamic
        url.query_pairs_mut().extend_pairs(&[
            ("q_album", track.album.name.as_str()),
            (
                "q_artist",
                track
                    .artists
                    .first()
                    .map(|artist| artist.name.as_str())
                    .unwrap_or_default(),
            ),
            ("q_artists", artists.as_str()),
            ("q_track", track.name.as_str()),
            (
                "track_spotify_id",
                track
                    .id
                    .as_ref()
                    .map(|id| id.to_string())
                    .unwrap_or_default()
                    .as_str(),
            ),
            (
                "q_duration",
                track.duration.as_secs_f64().to_string().as_str(),
            ),
            (
                "f_subtitle_length",
                track.duration.as_secs().to_string().as_str(),
            ),
            ("usertoken", usertoken.as_str()),
        ]);

        let res = self
            .reqwest
            .get(url)
            .header("authority", "apic-desktop.musixmatch.com")
            .header("cookie", "x-mxm-token-guid=")
            .send()
            .await;

        let text = res?.text().await?;

        let root: serde_json::Value = serde_json::from_str(&text)?;

        let lyrics = root["message"]["body"]["macro_calls"]["track.lyrics.get"]["message"]["body"]
            ["lyrics"]
            .clone();

        let lyrics: Lyrics = if let Value::Object(_) = lyrics {
            let res = from_value(lyrics);

            res?
        } else {
            return Ok(None);
        };

        let meta = root["message"]["body"]["macro_calls"]["matcher.track.get"]["message"]["body"]
            ["track"]
            .clone();

        Ok(Some(lyrics.lyrics))
    }
}
