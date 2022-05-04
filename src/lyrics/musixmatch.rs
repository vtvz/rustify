use std::collections::VecDeque;
use std::time::Duration;

use itertools::Itertools;
use reqwest::Client;
use rspotify::model::FullTrack;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{from_value, Value};
use teloxide::utils::markdown;
use tokio::sync::Mutex;

use crate::errors::{Context, GenericResult};

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
pub struct Lyrics {
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
    #[serde(default)]
    pub subtitle: Option<Vec<(Duration, String)>>,
    #[serde(rename = "lyrics_language")]
    pub language: String,
    #[serde(rename = "lyrics_language_description")]
    pub language_description: String,
    #[serde(rename = "backlink_url")]
    pub backlink_url: String,
}

impl super::SearchResult for Lyrics {
    fn provider(&self) -> super::Provider {
        super::Provider::Musixmatch
    }

    fn lyrics(&self) -> Vec<&str> {
        if let Some(subtitle) = &self.subtitle {
            subtitle.iter().map(|(_, text)| text.as_str()).collect()
        } else {
            self.lyrics.iter().map(|lyrics| lyrics.as_str()).collect()
        }
    }

    fn tg_link(&self, full: bool) -> String {
        let text = if full {
            "Musixmatch Source"
        } else {
            "Text truncated. Full lyrics can be found at Musixmatch"
        };

        format!(
            "[{text}]({url})",
            text = markdown::escape(text),
            url = self.backlink_url
        )
    }

    fn line_index_name(&self, index: usize) -> String {
        let Some(subtitle) = &self.subtitle else {
            return (index + 1).to_string()
        };

        let Some(line) = subtitle.get(index) else {
            return (index + 1).to_string()
        };

        let secs = line.0.as_secs();

        format!("{}:{:02}", secs / 60, secs % 60)
    }

    fn language(&self) -> &str {
        self.language.as_str()
    }
}

pub struct Musixmatch {
    reqwest: Client,
    tokens: Mutex<VecDeque<String>>,
}

impl Musixmatch {
    pub fn new(tokens: impl IntoIterator<Item = String>) -> Self {
        Self {
            reqwest: Client::new(),
            tokens: Mutex::new(tokens.into_iter().collect()),
        }
    }

    pub async fn search_for_track(&self, track: &FullTrack) -> GenericResult<Option<Lyrics>> {
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

        let usertoken = {
            let mut tokens = self.tokens.lock().await;
            tokens.rotate_left(1);

            tokens
                .front()
                .cloned()
                .context("Queue shouldn't be empty")?
        };

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

        let mut lyrics: Lyrics = if let Value::Object(_) = lyrics {
            from_value(lyrics)?
        } else {
            return Ok(None);
        };

        if lyrics.restricted || lyrics.instrumental {
            return Ok(None);
        }

        let subtitle_json = root["message"]["body"]["macro_calls"]["track.subtitles.get"]
            ["message"]["body"]["subtitle_list"][0]["subtitle"]["subtitle_body"]
            .clone();

        let subtitle_json = if let Value::String(subtitle_json) = subtitle_json {
            subtitle_json
        } else {
            return Ok(Some(lyrics));
        };

        if subtitle_json.is_empty() {
            return Ok(Some(lyrics));
        }

        let subtitle: Vec<Value> = match serde_json::from_str(&subtitle_json) {
            Ok(subtitle) => subtitle,
            Err(err) => {
                tracing::error!("Error with parsing track subtitles: {:?}", err);

                return Ok(Some(lyrics));
            },
        };

        let subtitle: Vec<(_, _)> = subtitle
            .iter()
            .filter_map(|line| {
                let Value::String(text) = line["text"].clone() else {
                    return None;
                };

                let Value::Number(total) = line["time"]["total"].clone() else {
                    return None;
                };

                let Some(total) = total.as_f64() else {
                    return None;
                };

                Some((Duration::from_secs_f64(total), text))
            })
            .collect();

        lyrics.subtitle = Some(subtitle);

        Ok(Some(lyrics))
    }
}
