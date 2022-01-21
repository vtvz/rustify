use crate::state::UserState;
use crate::CHECK_INTERVAL;
use lazy_static::lazy_static;
use rand::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::TrackId;
use std::collections::HashSet;
use std::str::FromStr;

lazy_static! {
    static ref RICK: TrackId =
        TrackId::from_str("4cOdK2wGLETKBW3PvgPWqT").expect("Should be parsable");
    static ref BLACKLIST: HashSet<String> = dotenv::var("RICKROLL_BLACKLIST")
        .unwrap_or_default()
        .split(',')
        .map(ToOwned::to_owned)
        .collect();
    static ref ENABLED: bool = dotenv::var("RICKROLL_ENABLED")
        .unwrap_or_else(|_| "true".into())
        .eq("true");
}

pub async fn should(user_id: &str, play: bool) -> bool {
    if !*ENABLED {
        return false;
    }

    if BLACKLIST.contains(user_id) {
        return false;
    }

    // Once per 5 days
    let chance = 5 * (24 * 60 * 60) / CHECK_INTERVAL;

    let mut rng = thread_rng();
    let range = if play { chance / 8 } else { chance / 2 };
    let chance = rng.gen_range(0..range);

    chance == 0
}

pub async fn like(state: &UserState) {
    log::warn!("User {} was RickRolled in favorites!", state.user_id);

    let rick = vec![RICK.clone()];
    let spotify = state.spotify.read().await;

    spotify
        .current_user_saved_tracks_delete(rick.iter())
        .await
        .ok();

    spotify
        .current_user_saved_tracks_add(rick.iter())
        .await
        .ok();
}

pub async fn queue(state: &UserState) {
    log::warn!("User {} was RickRolled in queue!", state.user_id);
    let spotify = state.spotify.read().await;

    spotify.add_item_to_queue(&RICK.clone(), None).await.ok();
}
