use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init() -> anyhow::Result<()> {
    let tracing_init = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(true)
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .finish()
        .with(
            Targets::new()
                .with_target(
                    &env!("CARGO_PKG_NAME").replace('-', "_"),
                    tracing::Level::TRACE,
                )
                .with_target("teloxide", tracing::Level::INFO)
                .with_default(tracing::Level::WARN),
        )
        .try_init();

    match &tracing_init {
        Ok(_) => log::info!("tracing_subscriber::fmt::try_init success"),
        Err(_) => log::error!("tracing_subscriber::fmt::try_init error"),
    }

    Ok(tracing_init?)
}
