use std::sync::LazyLock;

use tokio::time::Instant;

pub mod influx;
pub mod influx_collector;
pub mod prometheus;
pub mod prometheus_collector;

static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);
