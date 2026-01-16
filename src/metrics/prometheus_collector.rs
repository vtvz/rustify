use std::sync::LazyLock;
use std::time::Duration;

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

    client
        .metrics()
        .track_status
        .with_label_values(&["disliked"])
        .set(disliked.cast_signed());
    client
        .metrics()
        .track_status
        .with_label_values(&["ignored"])
        .set(ignored.cast_signed());
    client
        .metrics()
        .track_status
        .with_label_values(&["skipped"])
        .set(skipped.cast_signed());
    client
        .metrics()
        .track_status
        .with_label_values(&["removed_collection"])
        .set(removed_collection);
    client
        .metrics()
        .track_status
        .with_label_values(&["removed_playlists"])
        .set(removed_playlists);

    client.metrics().lyrics_checked.set(lyrics_checked);
    client.metrics().lyrics_analyzed.set(lyrics_analyzed);
    client.metrics().lyrics_found.set(lyrics_found);
    client.metrics().lyrics_profane.set(lyrics_profane);
    client
        .metrics()
        .lyrics_source
        .with_label_values(&["genius"])
        .set(lyrics_genius);
    client
        .metrics()
        .lyrics_source
        .with_label_values(&["musixmatch"])
        .set(lyrics_musixmatch);
    client
        .metrics()
        .lyrics_source
        .with_label_values(&["lrclib"])
        .set(lyrics_lrclib);

    client
        .metrics()
        .tick_health_total
        .set(i64::try_from(tick_health_status.total).unwrap_or(0));
    client
        .metrics()
        .tick_health_unhealthy
        .set(i64::try_from(tick_health_status.unhealthy.len()).unwrap_or(0));
    client
        .metrics()
        .tick_health_lagging
        .set(i64::try_from(tick_health_status.lagging.len()).unwrap_or(0));

    client
        .metrics()
        .spotify_rate_limit_errors
        .set(spotify_429_count.cast_signed());

    client
        .metrics()
        .uptime
        .set(START_TIME.elapsed().as_secs().cast_signed());

    // Update users by status
    for status in UserStatus::iter() {
        let users = UserService::count_users(app.db(), Some(status)).await?;
        client
            .metrics()
            .users_by_status
            .with_label_values(&[&status.to_value()])
            .set(users);
    }

    for (language, count) in TrackLanguageStatsService::stats_all_users(app.db(), None).await? {
        let lang_code = language.map_or("none", |language| language.to_639_3());
        client
            .metrics()
            .tracks_by_language
            .with_label_values(&[lang_code])
            .set(count);
    }

    client.push().await?;

    Ok(())
}

pub async fn collect_user_timings(
    client: &PrometheusClient,
    report: CheckReport,
) -> anyhow::Result<()> {
    client
        .metrics()
        .process_duration
        .observe(report.users_process_time.as_secs_f64());
    client
        .metrics()
        .max_process_duration
        .set(report.max_process_time.as_secs_f64());

    client
        .metrics()
        .process_users_count
        .inc_by(report.users_count as _);
    client
        .metrics()
        .process_users_processed
        .inc_by(report.users_processed as _);
    client
        .metrics()
        .process_parallel_count
        .set(i64::try_from(report.parallel_count).unwrap_or(0));

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
