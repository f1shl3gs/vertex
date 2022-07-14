mod nodes;

use async_trait::async_trait;
use bytes::Buf;
use event::Metric;
use framework::config::{DataType, GenerateConfig, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient};
use framework::sink::util::sink::Response;
use framework::Source;
use hyper::Body;
use serde::{Deserialize, Serialize};

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
        todo!()
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
    async fn collect(&self) -> Vec<Metric> {
        todo!()
    }

    async fn fetch<'a, T: Deserialize<'a>>(&self, path: &str) -> Result<T, crate::Error> {
        let mut builder = http::Request::get(format!("{}{}", self.endpoint, path));

        if let Some(auth) = &self.auth {
            builder = auth.apply_builder(builder);
        }

        let resp = self.http_client.send(builder.body(Body::empty())?).await?;
        if resp.is_successful() {
            return Err("Unexpected status code".into());
        }

        let body = hyper::body::to_bytes(resp.into_body()).await?;

        serde_json::from_reader(body.reader()).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }
}
