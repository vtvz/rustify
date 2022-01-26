use std::collections::HashSet;
use std::ops::Deref;
use std::str::FromStr;

use lazy_static::lazy_static;
use rand::prelude::*;
use rspotify::clients::OAuthClient;
use rspotify::model::{Id, TrackId};
use rspotify::AuthCodeSpotify;
use teloxide::requests::{Request, Requester};
use teloxide::utils::markdown;
use tokio::sync::Mutex;

use crate::state::UserState;
use crate::tick;

lazy_static! {
    static ref ENABLED: bool = dotenv::var("RICKROLL_ENABLED")
        .unwrap_or_else(|_| "true".into())
        .eq("true");
    static ref TRACKS: Vec<TrackId> = dotenv::var("RICKROLL_TRACKS")
        .unwrap_or_else(|_| "4cOdK2wGLETKBW3PvgPWqT".into())
        .split(',')
        .map(|id| TrackId::from_str(id).expect("id should be parsable"))
        .collect();
    static ref BLACKLIST: HashSet<String> = dotenv::var("RICKROLL_BLACKLIST")
        .unwrap_or_default()
        .split(',')
        .map(ToOwned::to_owned)
        .collect();
    static ref REPORT: Option<i64> = dotenv::var("RICKROLL_REPORT")
        .map(|res| res.parse().expect("Reporter should be valid chat number"))
        .ok();
    static ref RND: Mutex<StdRng> = Mutex::new(StdRng::from_entropy());
}

pub async fn should(user_id: &str, play: bool) -> bool {
    if !*ENABLED {
        return false;
    }

    if BLACKLIST.contains(user_id) {
        return false;
    }

    let range = if play {
        2 * 60 * 60 // once per 2 hours
    } else {
        2 * (24 * 60 * 60) // once per 2 days
    } / tick::CHECK_INTERVAL;

    let chance = RND.lock().await.gen_range(0..range);

    chance == 0
}

async fn pick() -> TrackId {
    let variant = RND.lock().await.gen_range(0..TRACKS.len());

    TRACKS.get(variant).cloned().expect("Should exist")
}

pub async fn like(state: &UserState) {
    let spotify = state.spotify.read().await;
    let variant = pick().await;

    let rick = vec![variant.clone()];

    let del = spotify.current_user_saved_tracks_delete(rick.iter()).await;

    if let Err(err) = del {
        report(
            state,
            spotify.deref(),
            &variant,
            format!("remove {:?}", err).as_str(),
        )
        .await;

        return;
    }

    let add = spotify.current_user_saved_tracks_add(rick.iter()).await;

    if let Err(err) = add {
        report(
            state,
            spotify.deref(),
            &variant,
            format!("add {:?}", err).as_str(),
        )
        .await;

        return;
    }

    report(state, spotify.deref(), &variant, "favorites").await;
}

pub async fn queue(state: &UserState) {
    let spotify = state.spotify.read().await;

    let variant = pick().await;
    let queue = spotify.add_item_to_queue(&variant, None).await;

    if let Err(err) = queue {
        report(
            state,
            spotify.deref(),
            &variant,
            format!("queue {:?}", err).as_str(),
        )
        .await;

        return;
    }

    report(state, spotify.deref(), &variant, "queue").await;
}

async fn report(state: &UserState, spotify: &AuthCodeSpotify, track_id: &TrackId, wher: &str) {
    let message = format!(
        "User {} {} was RickRolled in {} with https://open.spotify.com/track/{}!",
        state.user_id,
        spotify
            .me()
            .await
            .map(|res| res.display_name.unwrap_or_default())
            .map(|res| markdown::escape(res.as_str()))
            .unwrap_or_default(),
        markdown::escape(wher),
        track_id.id()
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
