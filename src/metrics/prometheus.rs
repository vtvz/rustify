use anyhow::{Context, anyhow};
use prometheus::{
    BasicAuthentication,
    Gauge,
    Histogram,
    IntCounter,
    IntGauge,
    IntGaugeVec,
    Registry,
    labels,
    register_gauge_with_registry,
    register_histogram_with_registry,
    register_int_counter_with_registry,
    register_int_gauge_vec_with_registry,
    register_int_gauge_with_registry,
};

#[derive(Debug)]
pub struct PrometheusClient {
    url: url::Url,
    job: String,
    instance: String,
    basic_auth: Option<(String, String)>,
    registry: Registry,
    metrics: PrometheusMetrics,
}

#[derive(Debug)]
pub struct PrometheusMetrics {
    pub track_status: IntGaugeVec,
    pub lyrics_checked: IntGauge,
    pub lyrics_analyzed: IntGauge,
    pub lyrics_found: IntGauge,
    pub lyrics_profane: IntGauge,
    pub lyrics_source: IntGaugeVec,
    pub process_duration: Histogram,
    pub process_check_interval_seconds: Gauge,
    pub process_users_checked: IntCounter,
    pub process_users_processed: IntCounter,
    pub process_parallel_threads: IntGauge,
    pub ticks: IntGauge,
    pub ticks_unhealthy: IntGauge,
    pub ticks_lagging: IntGauge,
    pub spotify_rate_limit_errors: IntGauge,
    pub users_by_status: IntGaugeVec,
    pub tracks_by_language: IntGaugeVec,
    pub uptime: IntGauge,
}

impl PrometheusClient {
    pub fn new(
        pushgateway_url: &str,
        job: &str,
        instance: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> anyhow::Result<Self> {
        let url = url::Url::parse(pushgateway_url).context("Invalid Pushgateway URL format")?;

        let basic_auth = username.map(|u| (u.to_owned(), password.unwrap_or_default().to_owned()));

        let registry = Registry::new_custom(
            Some("rustify".into()),
            Some(labels! {
                "app".to_owned() => "rustify".to_owned(),
            }),
        )?;

        let metrics = PrometheusMetrics {
            track_status: register_int_gauge_vec_with_registry!(
                "track_status_total",
                "Total tracks by status",
                &["status"],
                registry
            )
            .context("Failed to register track_status metric")?,

            lyrics_checked: register_int_gauge_with_registry!(
                "lyrics_checked_total",
                "Total lyrics checked",
                registry
            )
            .context("Failed to register lyrics_checked metric")?,

            lyrics_analyzed: register_int_gauge_with_registry!(
                "lyrics_analyzed_total",
                "Total lyrics analyzed",
                registry
            )
            .context("Failed to register lyrics_analyzed metric")?,

            lyrics_found: register_int_gauge_with_registry!(
                "lyrics_found_total",
                "Total lyrics found",
                registry
            )
            .context("Failed to register lyrics_found metric")?,

            lyrics_profane: register_int_gauge_with_registry!(
                "lyrics_profane_total",
                "Total profane lyrics",
                registry
            )
            .context("Failed to register lyrics_profane metric")?,

            lyrics_source: register_int_gauge_vec_with_registry!(
                "lyrics_source_total",
                "Lyrics by source",
                &["source"],
                registry
            )
            .context("Failed to register lyrics_source metric")?,

            process_duration: register_histogram_with_registry!(
                "process_duration_seconds",
                "Processing duration in seconds",
                vec![0.05, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0],
                registry
            )
            .context("Failed to register process_duration metric")?,

            process_check_interval_seconds: register_gauge_with_registry!(
                "process_check_interval_seconds",
                "Interval between user check cycles in seconds",
                registry
            )
            .context("Failed to register process_check_interval_seconds metric")?,

            process_users_checked: register_int_counter_with_registry!(
                "process_users_checked_total",
                "Number of users checked",
                registry
            )
            .context("Failed to register process_users_checked_total metric")?,

            process_users_processed: register_int_counter_with_registry!(
                "process_users_processed_total",
                "Total users processed",
                registry
            )
            .context("Failed to register process_users_processed_total metric")?,

            process_parallel_threads: register_int_gauge_with_registry!(
                "process_parallel_threads",
                "Number of parallel processing threads",
                registry
            )
            .context("Failed to register process_parallel_threads metric")?,

            ticks: register_int_gauge_with_registry!(
                "ticks",
                "Total number of active ticks",
                registry
            )
            .context("Failed to register ticks metric")?,

            ticks_unhealthy: register_int_gauge_with_registry!(
                "ticks_unhealthy",
                "Unhealthy tick count",
                registry
            )
            .context("Failed to register ticks_unhealthy metric")?,

            ticks_lagging: register_int_gauge_with_registry!(
                "ticks_lagging",
                "Lagging tick count",
                registry
            )
            .context("Failed to register ticks_lagging metric")?,

            spotify_rate_limit_errors: register_int_gauge_with_registry!(
                "errors_spotify_rate_limit_total",
                "Total Spotify 429 rate limit errors",
                registry
            )
            .context("Failed to register spotify_rate_limit_errors metric")?,

            users_by_status: register_int_gauge_vec_with_registry!(
                "users_by_status_total",
                "Users by status",
                &["status"],
                registry
            )
            .context("Failed to register users_by_status metric")?,

            tracks_by_language: register_int_gauge_vec_with_registry!(
                "tracks_by_language_total",
                "Tracks by language",
                &["language"],
                registry
            )
            .context("Failed to register tracks_by_language metric")?,

            uptime: register_int_gauge_with_registry!(
                "uptime_seconds",
                "Application uptime in seconds",
                registry
            )
            .context("Failed to register uptime metric")?,
        };

        Ok(Self {
            url,
            basic_auth,
            registry,
            metrics,
            job: job.to_owned(),
            instance: instance.to_owned(),
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn push(&self) -> anyhow::Result<()> {
        let url = self.url.clone();
        let job = self.job.clone();
        let instance = self.instance.clone();
        let auth = self.basic_auth.clone().map(|ba| BasicAuthentication {
            username: ba.0,
            password: ba.1,
        });

        let registry = self.registry.clone();

        tokio::task::spawn_blocking(move || {
            let metric_families = registry.gather();

            let grouping = labels! {
                "instance".to_owned() => instance,
            };

            prometheus::push_metrics(&job, grouping, url.as_ref(), metric_families, auth)
                .map_err(|e| anyhow!("Failed to push metrics: {e}"))
        })
        .await
        .context("Spawning blocking task failed")?
    }

    #[must_use]
    pub fn metrics(&self) -> &PrometheusMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid_url() {
        let client = PrometheusClient::new(
            "http://localhost:9091",
            "test_job",
            "test_instance",
            None,
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_invalid_url() {
        let client = PrometheusClient::new("not-a-url", "test_job", "test_instance", None, None);
        assert!(client.is_err());
    }

    #[test]
    fn test_new_with_auth() {
        let client = PrometheusClient::new(
            "http://localhost:9091",
            "test_job",
            "test_instance",
            Some("user"),
            Some("pass"),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert!(client.basic_auth.is_some());
    }
}
