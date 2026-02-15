use std::time::Duration;

use chrono::Utc;
use influxdb::{InfluxDbWriteable, Timestamp};
use sea_orm::{ActiveEnum as _, Iterable as _};
use tokio::sync::broadcast::error::RecvError;
use tracing::Instrument as _;

use crate::app::App;
use crate::entity::prelude::*;
use crate::metrics::influx::InfluxClient;
use crate::services::{
    MetricsService,
    TrackLanguageStatsService,
    TrackStatusService,
    UserService,
    UserStats,
};
use crate::tick::{CheckReport, PROCESS_TIME_CHANNEL};
use crate::utils;

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
    analyzed: u64,
    found: u64,
    profane: u64,
    genius: u64,
    musixmatch: u64,
    lrclib: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct TimingsStats {
    time: Timestamp,
    check_interval: u64,
    users_process_time: u64,
    users_checked: u64,
    users_processed: u64,
    threads_count: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct TickHealthStats {
    time: Timestamp,
    total_count: u64,
    unhealthy_count: u64,
    lagging_count: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct ErrorsStats {
    time: Timestamp,
    spotify_429: u64,
}

#[derive(InfluxDbWriteable, Debug)]
struct UsersStatusStats {
    time: Timestamp,
    count: u64,
    #[influxdb(tag)]
    status: String,
}

#[derive(InfluxDbWriteable, Debug)]
struct TrackLanguageStats {
    time: Timestamp,
    count: u64,
    #[influxdb(tag)]
    language: String,
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
            secs_elapsed: super::START_TIME.elapsed().as_secs(),
        }
    }
}

pub async fn collect(client: &InfluxClient, app: &App) -> anyhow::Result<()> {
    let disliked =
        TrackStatusService::count_status(app.db(), TrackStatus::Disliked, None, None).await?;
    let ignored =
        TrackStatusService::count_status(app.db(), TrackStatus::Ignore, None, None).await?;
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
        lyrics_analyzed,
    } = UserService::get_stats(app.db(), None).await?;

    let tick_health_status = utils::tick_health().await;

    let time = Timestamp::Seconds(Utc::now().timestamp() as u128);

    let mut redis_conn = app.redis_conn().await?;

    let mut metrics = vec![
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
            analyzed: lyrics_analyzed as u64,
            found: lyrics_found as u64,
            profane: lyrics_profane as u64,
            genius: lyrics_genius as u64,
            musixmatch: lyrics_musixmatch as u64,
            lrclib: lyrics_lrclib as u64,
        }
        .into_query("lyrics"),
        TickHealthStats {
            time,
            total_count: tick_health_status.total as u64,
            unhealthy_count: tick_health_status.unhealthy.len() as u64,
            lagging_count: tick_health_status.lagging.len() as u64,
        }
        .into_query("tick_health"),
        ErrorsStats {
            time,
            spotify_429: MetricsService::spotify_429_get(&mut redis_conn).await?,
        }
        .into_query("errors"),
        Uptime::new(time).into_query("uptime"),
    ];

    for status in UserStatus::iter() {
        let users = UserService::count_users(app.db(), Some(status)).await?;
        metrics.push(
            UsersStatusStats {
                time,
                count: users as u64,
                status: status.to_value(),
            }
            .into_query("user_status"),
        );
    }

    for (language, count) in TrackLanguageStatsService::stats_all_users(app.db(), None).await? {
        metrics.push(
            TrackLanguageStats {
                time,
                language: language
                    .map_or("none", |language| language.to_639_3())
                    .into(),
                count: count.try_into().unwrap_or_default(),
            }
            .into_query("track_language"),
        );
    }

    client
        .write(metrics.into_iter())
        .await?
        .error_for_status()?;

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
        check_interval: report.check_interval.as_millis() as u64,
        users_checked: report.users_checked as u64,
        users_processed: report.users_processed as u64,
        threads_count: report.threads_count as u64,
    }
    .into_query("process_timings");

    client.write([timings_stats]).await?.error_for_status()?;

    Ok(())
}

pub async fn collect_daemon(app: &'static App) {
    let Some(client) = app.influx() else {
        tracing::info!("Metrics collection disabled");

        return;
    };

    let _ = *super::START_TIME;

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
                () = utils::ctrl_c() => { return },
            }
        }
    }.in_current_span());

    utils::tick!(Duration::from_secs(60), {
        if let Err(err) = collect(client, app).await {
            tracing::error!(err = ?err, "Something went wrong on metrics collection");
        }
    });
}
