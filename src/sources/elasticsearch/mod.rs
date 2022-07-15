mod cluster_info;
mod nodes;

use async_trait::async_trait;
use event::Metric;
use framework::config::{DataType, GenerateConfig, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient};
use framework::sink::util::sink::Response;
use framework::{Pipeline, ShutdownSignal, Source};
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::time::Interval;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    endpoint: String,
    #[serde(default)]
    nodes: Vec<String>,
    #[serde(default)]
    auth: Option<Auth>,
}

impl GenerateConfig for Config {
    fn generate_config() -> String {
        todo!()
    }
}

#[async_trait]
#[typetag::serde(name = "elasticsearch")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let interval = tokio::time::interval(cx.interval);
        let http_client = HttpClient::new(None, &cx.proxy)?;
        let es = Elasticsearch {
            endpoint: self.endpoint.clone(),
            http_client,
            auth: self.auth.clone(),
            nodes: self.nodes.clone(),
        };

        Ok(Box::pin(es.run(interval, cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "elasticsearch"
    }
}

struct Elasticsearch {
    endpoint: String,
    http_client: HttpClient,
    auth: Option<Auth>,
    nodes: Vec<String>,
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
                            ?err
                        );

                        return Err(())
                    }
                }
            }
        }
    }

    async fn collect(&self) -> Vec<Metric> {
        let metrics = self.node_stats("_all").await;

        metrics
    }

    async fn fetch<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, crate::Error> {
        let mut builder = http::Request::get(format!("{}{}", self.endpoint, path));

        if let Some(auth) = &self.auth {
            builder = auth.apply_builder(builder);
        }

        let resp = self.http_client.send(builder.body(Body::empty())?).await?;
        if !resp.is_successful() {
            return Err("Unexpected status code".into());
        }

        let body = hyper::body::to_bytes(resp.into_body()).await?;

        serde_json::from_slice(&body).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use framework::config::ProxyConfig;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[tokio::test]
    async fn collect() {
        let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
        let es = Elasticsearch {
            endpoint: "http://localhost:9200".to_string(),
            http_client,
            auth: None,
            nodes: vec!["_all".to_string()],
        };

        let ms = es.collect().await;
        assert!(ms.len() > 2);
    }
}
