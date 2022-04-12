use chrono::Utc;
use influx::InfluxClient;
use influxdb::{InfluxDbWriteable, Timestamp};
use std::time::Duration;

use crate::track_status_service::{Status, TrackStatusService};
use crate::{tick, AppState};

pub mod influx;

#[derive(InfluxDbWriteable, Debug)]
struct TrackStatusStats {
    time: Timestamp,
    disliked: u32,
    ignored: u32,
    skipped: u32,
}

#[derive(InfluxDbWriteable, Debug)]
struct TimingsStats {
    time: Timestamp,
    max_process_time: u64,
    users_process_time: u64,
}

pub async fn collect(client: &InfluxClient, app_state: &AppState) -> anyhow::Result<()> {
    let disliked =
        TrackStatusService::count_status(&app_state.db, Status::Disliked, None, None).await? as u32;
    let ignored =
        TrackStatusService::count_status(&app_state.db, Status::Ignore, None, None).await? as u32;
    let skipped = TrackStatusService::sum_skips(&app_state.db, None).await?;

    // Let's write some data into a measurement called `weather`
    let time = Timestamp::Seconds(Utc::now().timestamp() as u128);

    let mut metrics = vec![];
    let track_status_stats = TrackStatusStats {
        time,
        disliked,
        ignored,
        skipped,
    }
    .into_query("track_status");

    metrics.push(track_status_stats);

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

    let mut interval = tokio::time::interval(Duration::from_secs(60));

    while !app_state.is_shutting_down().await {
        tokio::select! {
            _ = interval.tick() => {},
            _ = tokio::signal::ctrl_c() => {
                return;
            },
        }

        if let Err(err) = collect(client, app_state).await {
            tracing::error!(err = ?err, "Something went wrong on metrics collection: {:?}", err);
        }
    }
}
