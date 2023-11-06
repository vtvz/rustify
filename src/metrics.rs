use std::time::Duration;

use chrono::Utc;
use influx::InfluxClient;
use influxdb::{InfluxDbWriteable, Timestamp};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use tracing::Instrument;

use crate::entity::prelude::*;
use crate::tick::{CheckReport, PROCESS_TIME_CHANNEL};
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};
use crate::{utils, AppState};

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
    users_count: u64,
    users_checked: u64,
    parallel_count: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct TickHealthStats {
    time: Timestamp,
    total_count: u64,
    unhealthy_count: u64,
    lagging_count: u64,
}

lazy_static::lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[derive(InfluxDbWriteable, Debug)]
struct Uptime {
    time: Timestamp,
    secs_elapsed: u64,
}

impl Uptime {
    fn new(time: Timestamp) -> Self {
        Self {
            time,
            secs_elapsed: START_TIME.elapsed().as_secs(),
        }
    }
}

pub async fn collect(client: &InfluxClient, app_state: &AppState) -> anyhow::Result<()> {
    let disliked =
        TrackStatusService::count_status(app_state.db(), TrackStatus::Disliked, None, None).await?
            as u32;
    let ignored = TrackStatusService::count_status(app_state.db(), TrackStatus::Ignore, None, None)
        .await? as u32;
    let skipped = TrackStatusService::sum_skips(app_state.db(), None).await?;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_found,
        lyrics_profane,
        lyrics_genius,
        lyrics_musixmatch,
    } = UserService::get_stats(app_state.db(), None).await?;

    let tick_health_status = utils::tick_health().await;

    let time = Timestamp::Seconds(Utc::now().timestamp() as u128);

    let metrics = vec![
        TrackStatusStats {
            time,
            disliked,
            ignored,
            skipped,
            removed_collection,
            removed_playlists,
        }
        .into_query("track_status"),
        LyricsStats {
            time,
            checked: lyrics_checked,
            found: lyrics_found,
            profane: lyrics_profane,
            genius: lyrics_genius,
            musixmatch: lyrics_musixmatch,
        }
        .into_query("lyrics"),
        TickHealthStats {
            time,
            total_count: tick_health_status.total as u64,
            unhealthy_count: tick_health_status.unhealthy.len() as u64,
            lagging_count: tick_health_status.lagging.len() as u64,
        }
        .into_query("tick_health"),
        Uptime::new(time).into_query("uptime"),
    ];

    client.write(metrics.into_iter()).await?;

    Ok(())
}

pub async fn collect_user_timings(
    client: &InfluxClient,
    report: CheckReport,
) -> anyhow::Result<()> {
    let time = Timestamp::Milliseconds(Utc::now().timestamp_millis() as u128);

    let timings_stats = TimingsStats {
        time,
        users_process_time: report.users_process_time.as_millis() as u64,
        max_process_time: report.max_process_time.as_millis() as u64,
        users_count: report.users_count as u64,
        users_checked: report.users_checked as u64,
        parallel_count: report.parallel_count as u64,
    }
    .into_query("process_timings");

    client.write([timings_stats].into_iter()).await?;

    Ok(())
}

pub async fn collect_daemon(app_state: &'static AppState) {
    let Some(ref client) = app_state.influx() else {
        return;
    };

    lazy_static::initialize(&START_TIME);

    tokio::spawn(async {
        let mut rx = PROCESS_TIME_CHANNEL.0.subscribe();
        loop {
            tokio::select! {
                timings = rx.recv() => {
                    let report: CheckReport = match timings {
                        Err(RecvError::Closed) => return,
                        Err(RecvError::Lagged(lag)) => {
                            tracing::warn!(lag, "Have a bit of lag here");
                            continue;
                        },
                        Ok(timings) => timings,
                    };

                    if let Err(err) = collect_user_timings(client, report).await {
                        tracing::error!(err = ?err, "Something went wrong on user timing metrics collection");
                    }
                },
                _ = utils::ctrl_c() => { return },
            }
        }
    }.in_current_span());

    utils::tick!(Duration::from_secs(60), {
        if let Err(err) = collect(client, app_state).await {
            tracing::error!(err = ?err, "Something went wrong on metrics collection");
        }
    });
}
