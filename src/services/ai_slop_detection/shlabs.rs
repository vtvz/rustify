use chrono::Duration;
use redis::AsyncTypedCommands as _;
use serde_json::json;

use crate::spotify::ShortTrack;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root {
    pub result: Result,
    pub response_time: i64,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prediction {
    #[serde(rename = "Human Made")]
    HumanMade,
    #[serde(rename = "Pure AI")]
    PureAI,
    #[serde(rename = "Processed AI")]
    ProcessedAI,
    #[serde(rename = "Processed AI Generated")]
    ProcessedAIGenerated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Result {
    pub duration: f64,
    pub probability_ai_generated: f64,
    pub prediction: Prediction,
    pub confidence_score: Option<f64>,
    pub spectral_probabilities: Probabilities,
    pub temporal_probabilities: Probabilities,
    pub most_likely_ai_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Probabilities {
    pub human: f64,
    pub processed_ai: f64,
    pub pure_ai: f64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub daily_remaining: i64,
    pub monthly_remaining: i64,
}

const REDIS_KEY_ARTIST_PREFIX: &str = "rustify:ai_slop:shlabs:artist";

pub struct SHLabsProvider {
    client: reqwest::Client,
    api_key: String,
}

impl SHLabsProvider {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::builder()
                .timeout(
                    Duration::seconds(120)
                        .to_std()
                        .expect("It's positive. Will work"),
                )
                .build()
                .expect("Should work"),
        }
    }

    async fn fetch(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<Root> {
        let track_key = format!("{REDIS_KEY_ARTIST_PREFIX}:{}", track.id());

        if let Some(data) = redis_conn.get(&track_key).await?
            && let Ok(data) = serde_json::from_str(&data)
        {
            return Ok(data);
        }

        let res: Root = self
            .client
            .post("https://shlabs.music/api/v1/detect")
            .header("X-API-Key", &self.api_key)
            .json(&json!({
                "spotifyTrackId": track.id(),
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let _: () = redis_conn
            .set_ex(
                &track_key,
                serde_json::to_string(&res)?,
                Duration::days(365).num_seconds() as _,
            )
            .await?;

        Ok(res)
    }

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        let res = self.fetch(redis_conn, track).await?;

        Ok(!matches!(res.result.prediction, Prediction::HumanMade))
    }
}
