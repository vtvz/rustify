use rspotify::model::{FullTrack, Id, SimplifiedAlbum};
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
        r#"<a href="{link}">{name}</a>"#,
        name = html::escape(create_track_name(track).as_str()),
        link = track
            .external_urls
            .get("spotify")
            .map(String::as_str)
            .unwrap_or("https://vtvz.me/")
    )
}

pub fn create_album_tg_link(album: &SimplifiedAlbum) -> String {
    format!(
        r#"<a href="{link}">{name}</a>"#,
        name = html::escape(&album.name),
        link = album
            .external_urls
            .get("spotify")
            .map(String::as_str)
            .unwrap_or("https://vtvz.me/")
    )
}

pub fn create_track_name(track: &FullTrack) -> String {
    let artists = artist_names(track).join(", ");

    format!(r#"{} — {}"#, &artists, &track.name)
}
