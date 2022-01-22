use crate::state::UserState;
use crate::CHECK_INTERVAL;
use lazy_static::lazy_static;
use rand::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::TrackId;
use rspotify::AuthCodeSpotify;
use std::collections::HashSet;
use std::ops::Deref;
use std::str::FromStr;
use teloxide::requests::{Request, Requester};

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
    static ref REPORT: Option<i64> = dotenv::var("RICKROLL_REPORT")
        .map(|res| res.parse().expect("Reporter should be valid chat number"))
        .ok();
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

    report(state, spotify.deref(), "favorites").await;
}

pub async fn queue(state: &UserState) {
    let spotify = state.spotify.read().await;

    spotify.add_item_to_queue(&RICK.clone(), None).await.ok();

    report(state, spotify.deref(), "queue").await;
}

async fn report(state: &UserState, spotify: &AuthCodeSpotify, wher: &str) {
    let message = format!(
        "User {} {} was RickRolled in {wher}!",
        state.user_id,
        spotify
            .me()
            .await
            .map(|res| res.display_name.unwrap_or_default())
            .unwrap_or_default()
    );

    log::warn!("{}", message);

    let Some(report_id) = *REPORT else {
        return;
    };

    state
        .app
        .bot
        .send_message(report_id, message)
        .send()
        .await
        .ok();
}
