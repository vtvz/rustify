use async_trait::async_trait;
use chrono::{Duration, Timelike as _, Utc};
use redis::AsyncTypedCommands as _;
use serde_json::json;

use crate::services::ai_slop_detection::{AISlopDetectionPrediction, AISlopDetector};
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
    #[serde(rename = "Pure AI Generated")]
    PureAIGenerated,
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

const REDIS_KEY_TRACK_PREFIX: &str = "rustify:ai_slop:shlabs:track";
const REDIS_KEY_RATE_LIMITED: &str = "rustify:ai_slop:shlabs:rate_limited";

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

    async fn rate_limit(redis_conn: &mut deadpool_redis::Connection) -> anyhow::Result<()> {
        const SECONDS_HOUR: u64 = Duration::hours(1).num_seconds() as u64;

        let now = Utc::now();
        let seconds_until_midnight = (Duration::days(1)
            - Duration::seconds(i64::from(now.num_seconds_from_midnight())))
        .num_seconds()
        .max(1) as u64;

        let seconds_pause = seconds_until_midnight.min(SECONDS_HOUR);

        tracing::warn!(seconds_pause, "SHLabs rate limited, pausing");

        let _: () = redis_conn
            .set_ex(REDIS_KEY_RATE_LIMITED, 1, seconds_pause)
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip_all, fields(track_id = %track.id()))]
    async fn fetch(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<Option<Root>> {
        if redis_conn.exists(REDIS_KEY_RATE_LIMITED).await? {
            return Ok(None);
        }

        let track_key = format!("{REDIS_KEY_TRACK_PREFIX}:{}", track.id());

        if let Some(data) = redis_conn.get(&track_key).await?
            && let Ok(data) = serde_json::from_str(&data)
        {
            return Ok(Some(data));
        }

        let response = self
            .client
            .post("https://shlabs.music/api/v1/detect")
            .header("X-API-Key", &self.api_key)
            .json(&json!({
                "spotifyTrackId": track.id(),
            }))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            Self::rate_limit(redis_conn).await.ok();

            return Ok(None);
        }

        let res: Root = response.error_for_status()?.json().await?;

        let _: () = redis_conn
            .set_ex(
                &track_key,
                serde_json::to_string(&res)?,
                Duration::days(365).num_seconds() as _,
            )
            .await?;

        if res.usage.daily_remaining == 0 {
            Self::rate_limit(redis_conn).await.ok();
        }

        Ok(Some(res))
    }
}

#[async_trait]
impl AISlopDetector for SHLabsProvider {
    #[tracing::instrument(skip_all, fields(track_id = %track.id()))]
    async fn detect(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<AISlopDetectionPrediction> {
        let Some(res) = self.fetch(redis_conn, track).await? else {
            return Ok(AISlopDetectionPrediction::default());
        };

        Ok(match res.result.prediction {
            Prediction::HumanMade => AISlopDetectionPrediction::HumanMade,
            Prediction::PureAI | Prediction::PureAIGenerated => AISlopDetectionPrediction::PureAI,
            Prediction::ProcessedAI | Prediction::ProcessedAIGenerated => {
                AISlopDetectionPrediction::ProcessedAI
            },
        })
    }
}
