use std::time::Duration;

use anyhow::{Context, anyhow};
use prometheus::{
    BasicAuthentication,
    Histogram,
    IntGauge,
    IntGaugeVec,
    Registry,
    TextEncoder,
    labels,
    register_histogram_with_registry,
    register_int_gauge_vec_with_registry,
    register_int_gauge_with_registry,
};

#[derive(Debug)]
pub struct PrometheusClient {
    pushgateway_url: url::Url,
    basic_auth: Option<BasicAuthentication>,
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
    pub max_process_duration: Histogram,
    pub process_users_count: IntGauge,
    pub process_users_checked: IntGauge,
    pub process_parallel_count: IntGauge,
    pub tick_health_total: IntGauge,
    pub tick_health_unhealthy: IntGauge,
    pub tick_health_lagging: IntGauge,
    pub spotify_rate_limit_errors: IntGauge,
    pub users_by_status: IntGaugeVec,
    pub tracks_by_language: IntGaugeVec,
    pub uptime: IntGauge,
}

impl PrometheusClient {
    pub fn new(
        pushgateway_url: &str,
        job_name: &str,
        instance_tag: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
    ) -> anyhow::Result<Self> {
        // Validate URL format
        let url = url::Url::parse(pushgateway_url).context("Invalid Pushgateway URL format")?;

        // Ensure it's http or https
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(anyhow!("Pushgateway URL must use http or https scheme"));
        }

        let basic_auth = username.map(|u| BasicAuthentication {
            username: u.to_owned(),
            password: password.unwrap_or("").to_owned(),
        });

        let instance_tag = instance_tag.map_or_else(|| "unknown".into(), String::from);

        // Create custom registry
        let registry = Registry::new_custom(
            Some("rustify".into()),
            Some(labels! {
                "app".to_owned() => "rustify".to_owned(),
                "instance".to_owned() => instance_tag.clone(),
                "job".to_owned() => job_name.to_owned(),
            }),
        )?;

        // Register all metrics with the custom registry
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
                vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0],
                registry
            )
            .context("Failed to register process_duration metric")?,

            max_process_duration: register_histogram_with_registry!(
                "max_process_duration_seconds",
                "Max processing duration in seconds",
                vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0],
                registry
            )
            .context("Failed to register max_process_duration metric")?,

            process_users_count: register_int_gauge_with_registry!(
                "process_users_count",
                "Number of users processed",
                registry
            )
            .context("Failed to register process_users_count metric")?,

            process_users_checked: register_int_gauge_with_registry!(
                "process_users_checked_total",
                "Total users checked",
                registry
            )
            .context("Failed to register process_users_checked metric")?,

            process_parallel_count: register_int_gauge_with_registry!(
                "process_parallel_count",
                "Parallel processing count",
                registry
            )
            .context("Failed to register process_parallel_count metric")?,

            tick_health_total: register_int_gauge_with_registry!(
                "tick_health_total",
                "Total tick health count",
                registry
            )
            .context("Failed to register tick_health_total metric")?,

            tick_health_unhealthy: register_int_gauge_with_registry!(
                "tick_health_unhealthy_total",
                "Unhealthy tick count",
                registry
            )
            .context("Failed to register tick_health_unhealthy metric")?,

            tick_health_lagging: register_int_gauge_with_registry!(
                "tick_health_lagging_total",
                "Lagging tick count",
                registry
            )
            .context("Failed to register tick_health_lagging metric")?,

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
            pushgateway_url: url,
            basic_auth,
            registry,
            metrics,
        })
    }

    pub async fn push(&self) -> anyhow::Result<()> {
        let client = reqwest_compat::ClientBuilder::new()
            .timeout(Duration::from_secs(180))
            .build()?;

        let encoder = TextEncoder::new();

        let registry = self.registry.clone();
        let metric_families = registry.gather();
        let test = encoder.encode_to_string(&metric_families)?;

        println!("{test}");

        let response = client
            .post(self.pushgateway_url.clone())
            .body(test)
            .basic_auth(
                &self.basic_auth.as_ref().unwrap().username,
                self.basic_auth.as_ref().map(|item| &item.password),
            )
            .send()
            .await?;

        dbg!(&response);

        let text = response.text().await?;

        println!("{text}");

        Ok(())
    }

    // #[tracing::instrument(skip_all)]
    // pub async fn push2(&self) -> anyhow::Result<()> {
    //     let url = self.pushgateway_url.clone();
    //     let job = self.job_name.clone();
    //     let instance = self.instance_tag.clone();
    //     let auth = self.basic_auth.as_ref().map(|ba| BasicAuthentication {
    //         username: ba.username.clone(),
    //         password: ba.password.clone(),
    //     });
    //
    //     let registry = self.registry.clone();
    //
    //     tokio::task::spawn_blocking(move || {
    //         let metric_families = registry.gather();
    //
    //         let grouping = labels! {
    //             "app".to_owned() => "rustify".to_owned(),
    //             "instance".to_owned() => instance,
    //         };
    //
    //         prometheus::push_metrics(&job, grouping, &url.to_string(), metric_families, auth)
    //             .map_err(|e| anyhow!("Failed to push metrics: {e}"))
    //     })
    //     .await
    //     .context("Spawning blocking task failed")?
    // }

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
            Some("test_instance"),
            None,
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_invalid_url() {
        let client = PrometheusClient::new("not-a-url", "test_job", None, None, None);
        assert!(client.is_err());
    }

    #[test]
    fn test_new_with_auth() {
        let client = PrometheusClient::new(
            "http://localhost:9091",
            "test_job",
            None,
            Some("user"),
            Some("pass"),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert!(client.basic_auth.is_some());
    }

    #[test]
    fn test_new_invalid_scheme() {
        let client = PrometheusClient::new("ftp://localhost:9091", "test_job", None, None, None);
        assert!(client.is_err());
        assert!(
            client
                .unwrap_err()
                .to_string()
                .contains("http or https scheme")
        );
    }
}
