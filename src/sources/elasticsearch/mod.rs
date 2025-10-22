mod cluster_health;
mod cluster_info;
mod nodes;
mod slm;
mod snapshot;

use std::time::Duration;

use bytes::Bytes;
use configurable::configurable_component;
use event::Metric;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::http::{Auth, HttpClient};
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use tokio::time::Interval;

#[configurable_component(source, name = "elasticsearch")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Address of the Elasticsearch node we should connect to.
    endpoint: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default)]
    auth: Option<Auth>,

    /// Query stats for SLM.
    #[serde(default)]
    slm: bool,

    /// Query stats for the cluster snapshots.
    #[serde(default)]
    snapshots: bool,
    tls: Option<TlsConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "elasticsearch")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let interval = tokio::time::interval(self.interval);
        let http_client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;
        let es = Elasticsearch {
            endpoint: self.endpoint.clone(),
            http_client,
            auth: self.auth.clone(),
            slm: self.slm,
            snapshot: self.snapshots,
        };

        Ok(Box::pin(es.run(interval, cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

struct Elasticsearch {
    endpoint: String,
    http_client: HttpClient,
    auth: Option<Auth>,
    // nodes: Vec<String>,
    slm: bool,
    snapshot: bool,
}

impl Elasticsearch {
    async fn run(
        self,
        mut interval: Interval,
        mut output: Pipeline,
        mut shutdown: ShutdownSignal,
    ) -> Result<(), ()> {
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    return Ok(())
                },

                _ = interval.tick() => {
                    let metrics = self.collect().await;

                    if let Err(err) = output.send(metrics).await {
                        error!(
                            message = "Error sending metrics",
                            %err
                        );

                        return Err(())
                    }
                }
            }
        }
    }

    async fn collect(&self) -> Vec<Metric> {
        let mut metrics = self.node_stats("_all").await;

        metrics.extend(self.cluster_info().await);
        metrics.extend(self.cluster_health().await);

        if self.slm {
            metrics.extend(self.slm().await);
        }

        if self.snapshot {
            match self.snapshots().await {
                Ok(sm) => metrics.extend(sm),
                Err(err) => {
                    warn!(message = "Fetch snapshots metrics failed", ?err);
                }
            }
        }

        metrics
    }

    async fn fetch<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, crate::Error> {
        let mut builder = http::Request::get(format!("{}{}", self.endpoint, path));

        if let Some(auth) = &self.auth {
            builder = auth.apply_builder(builder);
        }

        let resp = self
            .http_client
            .send(builder.body(Full::<Bytes>::default())?)
            .await?;
        if !resp.status().is_success() {
            return Err("Unexpected status code".into());
        }

        let data = resp.into_body().collect().await?.to_bytes();

        serde_json::from_slice(&data).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use framework::config::ProxyConfig;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[tokio::test]
    async fn collect() {
        let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
        let es = Elasticsearch {
            endpoint: "http://localhost:9200".to_string(),
            http_client,
            auth: None,
            slm: false,
            snapshot: false, // nodes: vec!["_all".to_string()],
        };

        let ms = es.collect().await;
        assert!(ms.len() > 2);
    }
}
