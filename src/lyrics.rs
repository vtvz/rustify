use genius::GeniusLocal;
use musixmatch::Musixmatch;
use rspotify::model::FullTrack;

pub mod genius;
pub mod musixmatch;

pub trait SearchResult {
    fn lyrics(&self) -> Vec<&str>;
    fn tg_link(&self, full: bool) -> String;

    fn language(&self) -> &str {
        "en"
    }

    fn line_index_name(&self, index: usize) -> String {
        (index + 1).to_string()
    }
}

pub struct Manager {
    genius: GeniusLocal,
    musixmatch: Musixmatch,
}

impl Manager {
    pub fn new(genius_token: String, musixmatch_tokens: Vec<String>) -> Self {
        let genius = GeniusLocal::new(genius_token);
        let musixmatch = Musixmatch::new(musixmatch_tokens);
        Self { genius, musixmatch }
    }

    pub async fn search_for_track(
        &self,
        track: &FullTrack,
    ) -> anyhow::Result<Option<Box<dyn SearchResult + Send>>> {
        let musixmatch_result = self.musixmatch.search_for_track(track).await;

        let musixmatch_result = match musixmatch_result {
            Ok(Some(res)) => {
                return Ok(Some(Box::new(res)));
            }
            Err(err) => {
                tracing::error!("Error with Musixmatch occurred: {:?}", err);

                Err(err)
            }
            _ => {
                tracing::debug!("Musixmatch text not found for {:?}", track.id);

                Ok(None)
            }
        };

        if let Some(genius_result) = self.genius.search_for_track(track).await? {
            Ok(Some(Box::new(genius_result)))
        } else {
            musixmatch_result
        }
    }
}
