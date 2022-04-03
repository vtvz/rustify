use tracing_loki::Layer;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub async fn loki() -> anyhow::Result<Option<Layer>> {
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
        vec![].into_iter().collect(),
    )?;

    tokio::spawn(task);

    Ok(Some(layer))
}

pub async fn init() -> anyhow::Result<()> {
    let loki = loki().await?;

    let subscriber = tracing_subscriber::fmt()
        .with_file(false)
        .with_line_number(true)
        .without_time()
        .with_max_level(tracing::Level::TRACE)
        .finish();

    let builder = subscriber.with(loki).with(
        Targets::new()
            .with_target(
                &env!("CARGO_PKG_NAME").replace('-', "_"),
                tracing::Level::TRACE,
            )
            .with_target("teloxide", tracing::Level::INFO)
            .with_default(tracing::Level::WARN),
    );

    let tracing_init = builder.try_init();

    match &tracing_init {
        Ok(_) => log::info!("tracing_subscriber::fmt::try_init success"),
        Err(_) => log::error!("tracing_subscriber::fmt::try_init error"),
    }

    Ok(tracing_init?)
}
