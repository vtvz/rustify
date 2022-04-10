use std::time::Duration;
use tokio_retry::strategy::{jitter, ExponentialBackoff};
use tokio_retry::{Action, Retry};

pub fn retry<A>(action: A) -> Retry<Box<dyn Iterator<Item = Duration> + Send + Sync>, A>
where
    A: Action + Send + Sync,
{
    let strategy = ExponentialBackoff::from_millis(10)
        .max_delay(Duration::from_secs(30))
        .map(jitter)
        .take(5);

    Retry::spawn(
        Box::new(strategy) as Box<dyn Iterator<Item = Duration> + Send + Sync>,
        action,
    )
}
