mod shlabs;
mod soul_over_ai;
mod spotify_ai_blocker;

use async_trait::async_trait;
use chrono::NaiveDate;
use soul_over_ai::SoulOverAIProvider;
use spotify_ai_blocker::SpotifyAIBlockerProvider;

use crate::spotify::ShortTrack;

pub struct AISlopDetectionService {
    spotify_ai_blocker: SpotifyAIBlockerProvider,
    soul_over_ai: SoulOverAIProvider,
    shlabs: Option<shlabs::SHLabsProvider>,
}

pub enum Provider {
    SpotifyAIBlocker,
    SoulOverAI,
    SHLabs,
}

#[derive(Default)]
pub struct AISlopDetectionResult {
    pub provider: Option<Provider>,
    pub prediction: AISlopDetectionPrediction,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum AISlopDetectionPrediction {
    #[default]
    HumanMade,
    PureAI,
    ProcessedAI,
}

impl AISlopDetectionPrediction {
    #[must_use]
    pub fn is_track_ai(self) -> bool {
        self != Self::HumanMade
    }
}

#[async_trait]
pub trait AISlopDetector {
    async fn detect(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<AISlopDetectionPrediction>;
}

impl Provider {
    pub fn tg_link(&self) -> String {
        teloxide::utils::html::link(self.link(), self.name())
    }

    pub fn link(&self) -> &str {
        match self {
            Self::SpotifyAIBlocker => "https://github.com/CennoxX/spotify-ai-blocker",
            Self::SoulOverAI => "https://github.com/xoundbyte/soul-over-ai",
            Self::SHLabs => "https://www.submithub.com/ai-song-checker",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::SpotifyAIBlocker => "Spotify AI Music Blocker",
            Self::SoulOverAI => "Soul Over AI",
            Self::SHLabs => "SubmitHub AI Song Checker",
        }
    }
}

impl AISlopDetectionService {
    #[must_use]
    pub fn new(shlabs_api_key: Option<String>) -> Self {
        Self {
            spotify_ai_blocker: SpotifyAIBlockerProvider::new(),
            soul_over_ai: SoulOverAIProvider::new(),
            shlabs: shlabs_api_key.map(shlabs::SHLabsProvider::new),
        }
    }

    fn is_before_ai_era(date: Option<NaiveDate>) -> bool {
        if date.is_none() {
            return false;
        }

        date < NaiveDate::from_ymd_opt(2024, 1, 1)
    }

    #[tracing::instrument(skip_all, fields(track_id = %track.id()))]
    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<AISlopDetectionResult> {
        macro_rules! handle_provider {
            ($provider_enum:expr, $provider:expr) => {
                let result = AISlopDetector::detect($provider, redis_conn, track).await;

                match result {
                    Ok(prediction) => {
                        if prediction.is_track_ai() {
                            return Ok(AISlopDetectionResult {
                                provider: Some($provider_enum),
                                prediction,
                            });
                        }
                    },
                    Err(err) => {
                        tracing::error!(
                            err = ?err,
                            "Error with {} occurred",
                            $provider_enum.name()
                        );
                    },
                };
            };
        }

        if Self::is_before_ai_era(track.album_release_date()) {
            return Ok(AISlopDetectionResult::default());
        }

        handle_provider!(Provider::SoulOverAI, &self.soul_over_ai);
        handle_provider!(Provider::SpotifyAIBlocker, &self.spotify_ai_blocker);

        if let Some(shlabs) = &self.shlabs {
            handle_provider!(Provider::SHLabs, shlabs);
        }

        Ok(AISlopDetectionResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_before_ai_era() {
        assert!(AISlopDetectionService::is_before_ai_era(
            NaiveDate::from_ymd_opt(2023, 1, 1)
        ));
    }

    #[test]
    fn test_after_ai_era() {
        assert!(!AISlopDetectionService::is_before_ai_era(
            NaiveDate::from_ymd_opt(2024, 1, 1)
        ));
    }

    #[test]
    fn test_none_after_ai_era() {
        assert!(!AISlopDetectionService::is_before_ai_era(None));
    }
}
