use rspotify::model::{FullTrack, Id};
use teloxide::utils::html;

pub fn artist_names(track: &FullTrack) -> Vec<String> {
    track.artists.iter().map(|art| art.name.clone()).collect()
}

pub fn get_track_id(track: &FullTrack) -> String {
    track
        .id
        .as_ref()
        .map(|track_id| track_id.id().to_owned())
        .unwrap_or_default()
}

pub fn create_track_tg_link(track: &FullTrack) -> String {
    format!(
        r#"<a href="{}">{}</a>"#,
        html::escape(create_track_name(track).as_str()),
        track
            .external_urls
            .get("spotify")
            .cloned()
            .unwrap_or_else(|| "https://vtvz.me/".into())
    )
}

pub fn create_track_name(track: &FullTrack) -> String {
    let artists = artist_names(track).join(", ");

    format!(r#"{} — {}"#, &artists, &track.name)
}
