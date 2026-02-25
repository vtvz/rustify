mod spotify_ai_blocker;

use spotify_ai_blocker::SpotifyAIBlockerProvider;

use crate::spotify::ShortTrack;

pub struct AISlopDetectionService {
    spotify_ai_blocker_provider: SpotifyAIBlockerProvider,
}

impl AISlopDetectionService {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spotify_ai_blocker_provider: SpotifyAIBlockerProvider::new(),
        }
    }

    pub async fn is_track_ai(
        &self,
        redis_conn: &mut deadpool_redis::Connection,
        track: &ShortTrack,
    ) -> anyhow::Result<bool> {
        self.spotify_ai_blocker_provider
            .is_track_ai(redis_conn, track)
            .await
    }
}

impl Default for AISlopDetectionService {
    fn default() -> Self {
        Self::new()
    }
}
