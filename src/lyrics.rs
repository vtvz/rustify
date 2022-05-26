use genius::GeniusLocal;
use isolang::Language;
use musixmatch::Musixmatch;
use rspotify::model::FullTrack;
use strum_macros::Display;

use crate::errors::GenericResult;
use crate::spotify;

pub mod genius;
pub mod musixmatch;

#[derive(Display)]
pub enum Provider {
    Musixmatch,
    Genius,
}

pub trait SearchResult {
    fn provider(&self) -> Provider;
    fn lyrics(&self) -> Vec<&str>;
    fn tg_link(&self, full: bool) -> String;

    fn language(&self) -> Language;

    fn line_index_name(&self, index: usize) -> String {
        (index + 1).to_string()
    }
}

pub struct Manager {
    genius: GeniusLocal,
    musixmatch: Musixmatch,
}

impl Manager {
    pub fn new(genius_token: String, musixmatch_tokens: impl IntoIterator<Item = String>) -> Self {
        let genius = GeniusLocal::new(genius_token);
        let musixmatch = Musixmatch::new(musixmatch_tokens);
        Self { genius, musixmatch }
    }

    #[tracing::instrument(
        skip_all,
        fields(
            track_id = %spotify::utils::get_track_id(track),
            track_name = %spotify::utils::create_track_name(track),
        )
    )]
    pub async fn search_for_track(
        &self,
        track: &FullTrack,
    ) -> GenericResult<Option<Box<dyn SearchResult + Send>>> {
        let musixmatch_result = self.musixmatch.search_for_track(track).await;

        let musixmatch_result = match musixmatch_result {
            Ok(Some(res)) => {
                return Ok(Some(Box::new(res) as Box<dyn SearchResult + Send>));
            },
            Err(err) => {
                tracing::error!(
                    err = ?err,
                    "Error with Musixmatch occurred"
                );

                Err(err)
            },
            _ => {
                tracing::debug!("Musixmatch text not found");

                Ok(None)
            },
        };

        if let Some(genius_result) = self.genius.search_for_track(track).await? {
            Ok(Some(Box::new(genius_result)))
        } else {
            musixmatch_result
        }
    }
}
