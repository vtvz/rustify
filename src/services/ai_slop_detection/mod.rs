mod shlabs;
mod soul_over_ai;
mod spot_the_ai;
mod spotify_ai_blocker;

use soul_over_ai::SoulOverAIProvider;
use spot_the_ai::SpotTheAIProvider;
use spotify_ai_blocker::SpotifyAIBlockerProvider;

use crate::spotify::ShortTrack;

pub struct AISlopDetectionService {
    spotify_ai_blocker: SpotifyAIBlockerProvider,
    soul_over_ai: SoulOverAIProvider,
    spot_the_ai: SpotTheAIProvider,
    shlabs: Option<shlabs::SHLabsProvider>,
}

pub enum Provider {
    SpotifyAIBlocker,
    SoulOverAI,
    SpotTheAI,
    SHLabs,
}

pub struct AISlopDetectionResult {
    pub provider: Option<Provider>,
    pub is_track_ai: bool,
}

impl Provider {
    pub fn tg_link(&self) -> String {
        teloxide::utils::html::link(self.link(), self.name())
    }

    pub fn link(&self) -> &str {
        match self {
            Self::SpotifyAIBlocker => "https://github.com/CennoxX/spotify-ai-blocker",
            Self::SoulOverAI => "https://github.com/xoundbyte/soul-over-ai",
            Self::SpotTheAI => "https://spot-the-ai.com/list/",
            Self::SHLabs => "https://www.submithub.com/ai-song-checker",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::SpotifyAIBlocker => "Spotify AI Music Blocker",
            Self::SoulOverAI => "Soul Over AI",
            Self::SpotTheAI => "SpotAI",
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
            spot_the_ai: SpotTheAIProvider::new(),
            shlabs: shlabs_api_key.map(shlabs::SHLabsProvider::new),
        }
    }

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<AISlopDetectionResult> {
        macro_rules! handle_provider {
            ($provider_enum:expr, $provider:expr) => {
                let result = $provider.is_track_ai(redis_conn, track).await;

                match result {
                    Ok(res) => {
                        if res {
                            return Ok(AISlopDetectionResult {
                                provider: Some($provider_enum),
                                is_track_ai: res,
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

        handle_provider!(Provider::SpotifyAIBlocker, self.spotify_ai_blocker);
        handle_provider!(Provider::SoulOverAI, self.soul_over_ai);
        handle_provider!(Provider::SpotTheAI, self.spot_the_ai);

        if let Some(shlabs) = &self.shlabs {
            handle_provider!(Provider::SHLabs, shlabs);
        }

        Ok(AISlopDetectionResult {
            provider: None,
            is_track_ai: false,
        })
    }
}
