use anyhow::{Context, anyhow};
use prometheus::{BasicAuthentication, labels};

#[derive(Debug)]
pub struct PrometheusClient {
    pushgateway_url: String,
    job_name: String,
    instance_tag: String,
    basic_auth: Option<BasicAuthentication>,
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

        Ok(Self {
            pushgateway_url: pushgateway_url.to_owned(),
            job_name: job_name.to_owned(),
            instance_tag,
            basic_auth,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn push(&self) -> anyhow::Result<()> {
        let url = self.pushgateway_url.clone();
        let job = self.job_name.clone();
        let instance = self.instance_tag.clone();
        let auth = self.basic_auth.as_ref().map(|ba| BasicAuthentication {
            username: ba.username.clone(),
            password: ba.password.clone(),
        });

        // prometheus::push_metrics is blocking, so run in blocking task
        tokio::task::spawn_blocking(move || {
            // Gather all registered metrics
            let metric_families = prometheus::gather();

            // Create grouping labels
            let grouping = labels! {
                "app".to_owned() => "rustify".to_owned(),
                "instance".to_owned() => instance,
            };

            // Push to gateway
            prometheus::push_metrics(&job, grouping, &url, metric_families, auth)
                .map_err(|e| anyhow!("Failed to push metrics: {e}"))
        })
        .await
        .context("Spawning blocking task failed")?
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
