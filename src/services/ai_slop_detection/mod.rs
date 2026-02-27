mod soul_over_ai;
mod spot_the_ai;
mod spotify_ai_blocker;

use soul_over_ai::SoulOverAIProvider;
use spot_the_ai::SpotTheAIProvider;
use spotify_ai_blocker::SpotifyAIBlockerProvider;

use crate::spotify::ShortTrack;

pub struct AISlopDetectionService {
    spotify_ai_blocker_provider: SpotifyAIBlockerProvider,
    soul_over_ai_provider: SoulOverAIProvider,
    spot_the_ai: SpotTheAIProvider,
}

impl AISlopDetectionService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spotify_ai_blocker_provider: SpotifyAIBlockerProvider::new(),
            soul_over_ai_provider: SoulOverAIProvider::new(),
            spot_the_ai: SpotTheAIProvider::new(),
        }
    }

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        macro_rules! handle_provider {
            ($name:expr, $provider:expr) => {
                let result = $provider.is_track_ai(redis_conn, track).await;

                match result {
                    Ok(res) => {
                        if res {
                            return Ok(res);
                        }
                    },
                    Err(err) => {
                        tracing::error!(
                            err = ?err,
                            "Error with {} occurred",
                            $name
                        );
                    },
                };
            };
        }

        handle_provider!("Spotify AI Blocker", self.spotify_ai_blocker_provider);
        handle_provider!("Soul Over AI", self.soul_over_ai_provider);
        handle_provider!("Spot the AI", self.spot_the_ai);

        Ok(false)
    }
}

impl Default for AISlopDetectionService {
    fn default() -> Self {
        Self::new()
    }
}
