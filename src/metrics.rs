use chrono::Utc;
use influx::InfluxClient;
use influxdb::{InfluxDbWriteable, Timestamp};
use std::time::Duration;

use crate::entity::prelude::*;
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};
use crate::{tick, utils, AppState};

pub mod influx;

#[derive(InfluxDbWriteable, Debug)]
struct TrackStatusStats {
    time: Timestamp,
    disliked: u32,
    ignored: u32,
    skipped: u32,
    removed_collection: u32,
    removed_playlists: u32,
}

#[derive(InfluxDbWriteable, Debug)]
struct LyricsStats {
    time: Timestamp,
    checked: u32,
    found: u32,
    profane: u32,
    genius: u32,
    musixmatch: u32,
}

#[derive(InfluxDbWriteable, Debug)]
struct TimingsStats {
    time: Timestamp,
    max_process_time: u64,
    users_process_time: u64,
}

pub async fn collect(client: &InfluxClient, app_state: &AppState) -> anyhow::Result<()> {
    let disliked =
        TrackStatusService::count_status(&app_state.db, TrackStatus::Disliked, None, None).await?
            as u32;
    let ignored = TrackStatusService::count_status(&app_state.db, TrackStatus::Ignore, None, None)
        .await? as u32;
    let skipped = TrackStatusService::sum_skips(&app_state.db, None).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_found,
        lyrics_profane,
        lyrics_genius,
        lyrics_musixmatch,
    } = UserService::get_stats(&app_state.db, None).await?;

    let time = Timestamp::Seconds(Utc::now().timestamp() as u128);

    let mut metrics = vec![];
    let track_status_stats = TrackStatusStats {
        time,
        disliked,
        ignored,
        skipped,
        removed_collection,
        removed_playlists,
    }
    .into_query("track_status");

    metrics.push(track_status_stats);

    let lyrics_stats = LyricsStats {
        time,
        checked: lyrics_checked,
        found: lyrics_found,
        profane: lyrics_profane,
        genius: lyrics_genius,
        musixmatch: lyrics_musixmatch,
    }
    .into_query("lyrics");

    metrics.push(lyrics_stats);

    if let Some(timings) = *tick::PROCESS_TIME.lock().await {
        let timings_stats = TimingsStats {
            time,
            users_process_time: timings.as_millis() as u64,
            max_process_time: Duration::from_secs(tick::CHECK_INTERVAL).as_millis() as u64,
        }
        .into_query("process_timings");

        metrics.push(timings_stats);
    }

    client.write(metrics.into_iter()).await?;

    Ok(())
}

pub async fn collect_daemon(app_state: &AppState) {
    let Some(ref client) = app_state.influx else {
        return;
    };

    utils::tick!(Duration::from_secs(60), {
        if let Err(err) = collect(client, app_state).await {
            tracing::error!(err = ?err, "Something went wrong on metrics collection: {:?}", err);
        }
    });
}
