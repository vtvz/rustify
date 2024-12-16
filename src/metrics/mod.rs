use std::time::Duration;

use chrono::Utc;
use influx::InfluxClient;
use influxdb::{InfluxDbWriteable, Timestamp};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use tracing::Instrument;

use crate::entity::prelude::*;
use crate::state::AppState;
use crate::tick::{CheckReport, PROCESS_TIME_CHANNEL};
use crate::track_status_service::TrackStatusService;
use crate::user_service::{UserService, UserStats};
use crate::utils;

pub mod influx;

#[derive(InfluxDbWriteable, Debug)]
struct TrackStatusStats {
    time: Timestamp,
    disliked: u64,
    ignored: u64,
    skipped: u64,
    removed_collection: u64,
    removed_playlists: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct LyricsStats {
    time: Timestamp,
    checked: u64,
    found: u64,
    profane: u64,
    genius: u64,
    musixmatch: u64,
    lrclib: u64,
    azlyrics: u64,
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

pub async fn collect(client: &InfluxClient, app: &AppState) -> anyhow::Result<()> {
    let disliked =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, None, None).await? as u64;
    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, None, None).await? as u64;
    let skipped = TrackStatusService::sum_skips(app.db(), None).await? as u64;

    let UserStats {
        removed_collection,
        removed_playlists,
        lyrics_checked,
        lyrics_found,
        lyrics_profane,
        lyrics_genius,
        lyrics_musixmatch,
        lyrics_lrclib,
        lyrics_azlyrics,
    } = UserService::get_stats(app.db(), None).await?;

    let tick_health_status = utils::tick_health().await;

    let time = Timestamp::Seconds(Utc::now().timestamp() as u128);

    let metrics = vec![
        TrackStatusStats {
            time,
            disliked,
            ignored,
            skipped,
            removed_collection: removed_collection as u64,
            removed_playlists: removed_playlists as u64,
        }
        .into_query("track_status"),
        LyricsStats {
            time,
            checked: lyrics_checked as u64,
            found: lyrics_found as u64,
            profane: lyrics_profane as u64,
            genius: lyrics_genius as u64,
            musixmatch: lyrics_musixmatch as u64,
            lrclib: lyrics_lrclib as u64,
            azlyrics: lyrics_azlyrics as u64,
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

pub async fn collect_daemon(app: &'static AppState) {
    let Some(ref client) = app.influx() else {
        tracing::info!("Metrics collection disabled");

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
        if let Err(err) = collect(client, app).await {
            tracing::error!(err = ?err, "Something went wrong on metrics collection");
        }
    });
}
