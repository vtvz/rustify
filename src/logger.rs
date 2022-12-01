use std::collections::HashMap;
use std::str::FromStr;

use tracing_loki::Layer;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::errors::GenericResult;

pub async fn loki() -> GenericResult<Option<Layer>> {
    let Ok(loki_url) = dotenv::var("LOKI_URL") else {
        return Ok(None)
    };

    if loki_url.is_empty() {
        return Ok(None);
    }

    let (layer, task) = tracing_loki::layer(
        url::Url::parse(&loki_url)?,
        vec![
            ("app".into(), "rustify".into()),
            (
                "instance".into(),
                dotenv::var("LOKI_INSTANCE").unwrap_or_else(|_| "unknown".into()),
            ),
        ]
        .into_iter()
        .collect(),
        HashMap::new(),
    )?;

    tokio::spawn(task);

    Ok(Some(layer))
}

pub async fn init() -> GenericResult<()> {
    let loki = loki().await?;

    let subscriber = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(true)
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    let level = dotenv::var("LOG_LEVEL").unwrap_or_else(|_| "warn".into());

    let builder = subscriber.with(loki).with(
        Targets::new()
            .with_target(
                env!("CARGO_PKG_NAME").replace('-', "_"),
                tracing::Level::TRACE,
            )
            .with_target("teloxide", tracing::Level::INFO)
            .with_target("again", tracing::Level::TRACE)
            .with_default(tracing::Level::from_str(&level)?),
    );

    let tracing_init = builder.try_init();

    match &tracing_init {
        Ok(_) => log::info!("tracing_subscriber::fmt::try_init success"),
        Err(_) => log::error!("tracing_subscriber::fmt::try_init error"),
    }

    Ok(tracing_init?)
}
