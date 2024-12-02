use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Context;
use cached::proc_macro::io_cached;
use isolang::Language;
use itertools::Itertools;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value};
use tokio::sync::Mutex;

use crate::serde_utils::{bool_from_int, lines_from_string};
use crate::spotify::ShortTrack;

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Lyrics {
    #[serde(with = "bool_from_int")]
    pub verified: bool,
    #[serde(with = "bool_from_int")]
    pub restricted: bool,
    #[serde(with = "bool_from_int")]
    pub instrumental: bool,
    #[serde(with = "bool_from_int")]
    pub explicit: bool,
    #[serde(with = "lines_from_string", rename = "lyrics_body")]
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

    fn link(&self) -> String {
        self.backlink_url.clone()
    }

    fn link_text(&self, full: bool) -> String {
        let text = if full {
            "Musixmatch Source"
        } else {
            "Text truncated. Full lyrics can be found at Musixmatch"
        };

        text.into()
    }

    fn line_index_name(&self, index: usize) -> String {
        let Some(subtitle) = &self.subtitle else {
            return (index + 1).to_string();
        };

        let Some(line) = subtitle.get(index) else {
            return (index + 1).to_string();
        };

        let secs = line.0.as_secs();

        format!("{}:{:02}", secs / 60, secs % 60)
    }

    fn language(&self) -> Language {
        Language::from_639_1(&self.language).unwrap_or_default()
    }
}

pub struct Musixmatch {
    reqwest: Client,
    tokens: Mutex<VecDeque<String>>,
}

impl Musixmatch {
    pub fn new(tokens: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        Ok(Self {
            reqwest: ClientBuilder::new()
                .timeout(Duration::from_secs(5))
                .build()?,
            tokens: Mutex::new(tokens.into_iter().collect()),
        })
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = track.id(),
            track_name = track.name_with_artists(),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<Box<dyn super::SearchResult + Send>>> {
        #[io_cached(
            map_error = r##"|e| anyhow::Error::from(e) "##,
            convert = r#"{ track.id().into() }"#,
            ty = "cached::AsyncRedisCache<String, Option<Lyrics>>",
            create = r##" {
                let prefix = module_path!().split("::").last().expect("Will be");
                super::LyricsCacheManager::redis_cache_build(prefix).await.expect("Redis cache should build")
            } "##
        )]
        async fn search_for_track_middleware(
            musixmatch: &Musixmatch,
            track: &ShortTrack,
        ) -> anyhow::Result<Option<Lyrics>> {
            Musixmatch::search_for_track_internal(musixmatch, track).await
        }

        search_for_track_middleware(self, track)
            .await
            .map(|res| res.map(|opt| Box::new(opt) as _))
    }

    async fn search_for_track_internal(
        &self,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<Lyrics>> {
        let mut url =
            reqwest::Url::parse("https://apic-desktop.musixmatch.com/ws/1.1/macro.subtitles.get")?;

        // Static
        url.query_pairs_mut().extend_pairs(&[
            ("format", "json"),
            ("namespace", "lyrics_synched"),
            ("subtitle_format", "mxm"),
            ("app_id", "web-desktop-app-v1.0"),
        ]);

        let artists = track.artist_names().iter().join(",");

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
            ("q_album", track.album_name()),
            ("q_artist", track.first_artist_name()),
            ("q_artists", artists.as_str()),
            ("q_track", track.name()),
            ("track_spotify_id", track.id()),
            ("q_duration", track.duration_secs().to_string().as_str()),
            (
                "f_subtitle_length",
                track.duration_secs().to_string().as_str(),
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
                tracing::error!(err = ?err, "Error with parsing track subtitles");

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

                let total = total.as_f64()?;

                Some((Duration::from_secs_f64(total), text))
            })
            .collect();

        lyrics.subtitle = Some(subtitle);

        Ok(Some(lyrics))
    }
}
