use anyhow::anyhow;
use influxdb::Query;
use reqwest::{header, Response};

pub struct InfluxClient {
    req: reqwest::Client,
    url: reqwest::Url,
    instance_tag: String,
}

impl InfluxClient {
    pub fn new(
        api_url: &str,
        bucket: &str,
        org: &str,
        token: &str,
        instance_tag: Option<&str>,
    ) -> anyhow::Result<Self> {
        let headers = {
            let mut headers = header::HeaderMap::new();

            let mut auth_header = header::HeaderValue::from_str(&format!("Token {}", token))?;

            auth_header.set_sensitive(true);

            headers.insert(header::AUTHORIZATION, auth_header);

            headers
        };

        let req = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        let url = {
            let mut url = reqwest::Url::parse(api_url)?;

            url.query_pairs_mut()
                .append_pair("bucket", bucket)
                .append_pair("org", org);

            // Check beforehand
            url.path_segments_mut()
                .map_err(|_| anyhow!("Path segments should be parsable"))?;

            url
        };

        let instance_tag = instance_tag
            .map(String::from)
            .unwrap_or_else(|| "unknown".into());

        Ok(Self {
            req,
            url,
            instance_tag,
        })
    }

    fn write_url(&self) -> reqwest::Url {
        let mut url = self.url.clone();

        url.path_segments_mut()
            .expect("failed to get path segments")
            .push("write");

        url
    }

    pub async fn write<I>(&self, queries: I) -> anyhow::Result<Response>
    where
        I: Iterator<Item = influxdb::WriteQuery>,
    {
        let queries: Vec<_> = queries
            .map(|query| {
                query
                    .add_tag("app", "rustify")
                    .add_tag("instance", self.instance_tag.as_str())
            })
            .collect();

        if queries.is_empty() {
            return Err(anyhow!("No queries to write"));
        }

        let precision = queries[0].get_precision();

        let resp = self
            .req
            .post(self.write_url())
            .body(queries.build()?.get())
            .query(&[("precision", precision)])
            .send()
            .await?;

        Ok(resp.error_for_status()?)
    }
}
