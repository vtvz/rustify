mod soul_over_ai;
mod spotify_ai_blocker;

use soul_over_ai::SoulOverAIProvider;
use spotify_ai_blocker::SpotifyAIBlockerProvider;

use crate::spotify::ShortTrack;

pub struct AISlopDetectionService {
    spotify_ai_blocker_provider: SpotifyAIBlockerProvider,
    soul_over_ai_provider: SoulOverAIProvider,
}

impl AISlopDetectionService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spotify_ai_blocker_provider: SpotifyAIBlockerProvider::new(),
            soul_over_ai_provider: SoulOverAIProvider::new(),
        }
    }

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        if self
            .spotify_ai_blocker_provider
            .is_track_ai(redis_conn, track)
            .await?
        {
            return Ok(true);
        }

        self.soul_over_ai_provider
            .is_track_ai(redis_conn, track)
            .await
    }
}

impl Default for AISlopDetectionService {
    fn default() -> Self {
        Self::new()
    }
}
