use std::sync::LazyLock;
use std::time::Duration;

use prometheus::{
    Histogram,
    IntGauge,
    IntGaugeVec,
    register_histogram,
    register_int_gauge,
    register_int_gauge_vec,
};
use sea_orm::{ActiveEnum as _, Iterable as _};
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;
use tracing::Instrument;

use super::prometheus::PrometheusClient;
use crate::app::App;
use crate::entity::prelude::*;
use crate::services::{
    MetricsService,
    TrackLanguageStatsService,
    TrackStatusService,
    UserService,
    UserStats,
};
use crate::tick::{CheckReport, PROCESS_TIME_CHANNEL};
use crate::utils;

// Define Prometheus metrics using LazyLock
static TRACK_STATUS: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec!(
        "rustify_track_status_total",
        "Total tracks by status",
        &["status"]
    )
    .expect("Failed to register track_status metric")
});

static LYRICS_CHECKED: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_lyrics_checked_total", "Total lyrics checked")
        .expect("Failed to register lyrics_checked metric")
});

static LYRICS_ANALYZED: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_lyrics_analyzed_total", "Total lyrics analyzed")
        .expect("Failed to register lyrics_analyzed metric")
});

static LYRICS_FOUND: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_lyrics_found_total", "Total lyrics found")
        .expect("Failed to register lyrics_found metric")
});

static LYRICS_PROFANE: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_lyrics_profane_total", "Total profane lyrics")
        .expect("Failed to register lyrics_profane metric")
});

static LYRICS_SOURCE: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec!(
        "rustify_lyrics_source_total",
        "Lyrics by source",
        &["source"]
    )
    .expect("Failed to register lyrics_source metric")
});

static PROCESS_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustify_process_duration_seconds",
        "Processing duration in seconds",
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("Failed to register process_duration metric")
});

static MAX_PROCESS_DURATION: LazyLock<Histogram> = LazyLock::new(|| {
    register_histogram!(
        "rustify_max_process_duration_seconds",
        "Max processing duration in seconds",
        vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]
    )
    .expect("Failed to register max_process_duration metric")
});

static PROCESS_USERS_COUNT: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_process_users_count", "Number of users processed")
        .expect("Failed to register process_users_count metric")
});

static PROCESS_USERS_CHECKED: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_process_users_checked_total", "Total users checked")
        .expect("Failed to register process_users_checked metric")
});

static PROCESS_PARALLEL_COUNT: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustify_process_parallel_count",
        "Parallel processing count"
    )
    .expect("Failed to register process_parallel_count metric")
});

static TICK_HEALTH_TOTAL: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_tick_health_total", "Total tick health count")
        .expect("Failed to register tick_health_total metric")
});

static TICK_HEALTH_UNHEALTHY: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustify_tick_health_unhealthy_total",
        "Unhealthy tick count"
    )
    .expect("Failed to register tick_health_unhealthy metric")
});

static TICK_HEALTH_LAGGING: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_tick_health_lagging_total", "Lagging tick count")
        .expect("Failed to register tick_health_lagging metric")
});

static SPOTIFY_RATE_LIMIT_ERRORS: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!(
        "rustify_errors_spotify_rate_limit_total",
        "Total Spotify 429 rate limit errors"
    )
    .expect("Failed to register spotify_rate_limit_errors metric")
});

static USERS_BY_STATUS: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec!(
        "rustify_users_by_status_total",
        "Users by status",
        &["status"]
    )
    .expect("Failed to register users_by_status metric")
});

static TRACKS_BY_LANGUAGE: LazyLock<IntGaugeVec> = LazyLock::new(|| {
    register_int_gauge_vec!(
        "rustify_tracks_by_language_total",
        "Tracks by language",
        &["language"]
    )
    .expect("Failed to register tracks_by_language metric")
});

static UPTIME: LazyLock<IntGauge> = LazyLock::new(|| {
    register_int_gauge!("rustify_uptime_seconds", "Application uptime in seconds")
        .expect("Failed to register uptime metric")
});

static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

pub async fn collect(client: &PrometheusClient, app: &App) -> anyhow::Result<()> {
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

    let mut redis_conn = app.redis_conn().await?;
    let spotify_429_count = MetricsService::spotify_429_get(&mut redis_conn).await?;

    // Update Prometheus metrics
    TRACK_STATUS
        .with_label_values(&["disliked"])
        .set(disliked.cast_signed());
    TRACK_STATUS
        .with_label_values(&["ignored"])
        .set(ignored.cast_signed());
    TRACK_STATUS
        .with_label_values(&["skipped"])
        .set(skipped.cast_signed());
    TRACK_STATUS
        .with_label_values(&["removed_collection"])
        .set(removed_collection);
    TRACK_STATUS
        .with_label_values(&["removed_playlists"])
        .set(removed_playlists);

    LYRICS_CHECKED.set(lyrics_checked);
    LYRICS_ANALYZED.set(lyrics_analyzed);
    LYRICS_FOUND.set(lyrics_found);
    LYRICS_PROFANE.set(lyrics_profane);
    LYRICS_SOURCE
        .with_label_values(&["genius"])
        .set(lyrics_genius);
    LYRICS_SOURCE
        .with_label_values(&["musixmatch"])
        .set(lyrics_musixmatch);
    LYRICS_SOURCE
        .with_label_values(&["lrclib"])
        .set(lyrics_lrclib);

    TICK_HEALTH_TOTAL.set(i64::try_from(tick_health_status.total).unwrap_or(0));
    TICK_HEALTH_UNHEALTHY.set(i64::try_from(tick_health_status.unhealthy.len()).unwrap_or(0));
    TICK_HEALTH_LAGGING.set(i64::try_from(tick_health_status.lagging.len()).unwrap_or(0));

    SPOTIFY_RATE_LIMIT_ERRORS.set(spotify_429_count.cast_signed());

    UPTIME.set(START_TIME.elapsed().as_secs().cast_signed());

    // Update users by status
    for status in UserStatus::iter() {
        let users = UserService::count_users(app.db(), Some(status)).await?;
        USERS_BY_STATUS
            .with_label_values(&[&status.to_value()])
            .set(users);
    }

    // Update tracks by language
    for (language, count) in TrackLanguageStatsService::stats_all_users(app.db(), None).await? {
        let lang_code = language.map_or("none", |language| language.to_639_3());
        TRACKS_BY_LANGUAGE
            .with_label_values(&[lang_code])
            .set(count);
    }

    // Push all metrics to Pushgateway
    client.push().await?;

    Ok(())
}

pub async fn collect_user_timings(
    client: &PrometheusClient,
    report: CheckReport,
) -> anyhow::Result<()> {
    // Update histogram metrics
    PROCESS_DURATION.observe(report.users_process_time.as_secs_f64());
    MAX_PROCESS_DURATION.observe(report.max_process_time.as_secs_f64());

    PROCESS_USERS_COUNT.set(i64::try_from(report.users_count).unwrap_or(0));
    PROCESS_USERS_CHECKED.set(i64::try_from(report.users_checked).unwrap_or(0));
    PROCESS_PARALLEL_COUNT.set(i64::try_from(report.parallel_count).unwrap_or(0));

    // Push metrics
    client.push().await?;

    Ok(())
}

pub async fn collect_daemon(app: &'static App) {
    let Some(client) = app.prometheus() else {
        tracing::info!("Prometheus metrics collection disabled");

        return;
    };

    let _ = *START_TIME;

    tokio::spawn(
        async {
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
                            tracing::error!(err = ?err, "Something went wrong on Prometheus user timing metrics collection");
                        }
                    },
                    () = utils::ctrl_c() => { return },
                }
            }
        }
        .in_current_span(),
    );

    utils::tick!(Duration::from_secs(60), {
        if let Err(err) = collect(client, app).await {
            tracing::error!(err = ?err, "Something went wrong on Prometheus metrics collection");
        }
    });
}
