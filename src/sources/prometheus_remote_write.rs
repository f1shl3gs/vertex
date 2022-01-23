use std::net::SocketAddr;

use bytes::Bytes;
use event::Event;
use http::{HeaderMap, Method, StatusCode, Uri};
use prometheus::proto;
use prost::Message;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::config::{
    DataType, GenerateConfig, Output, Resource, SourceConfig, SourceContext, SourceDescription,
};
use crate::sources::utils::http::{decode, ErrorMessage};
use crate::sources::{
    utils::http::{HttpSource, HttpSourceAuthConfig},
    Source,
};
use crate::tls::TlsConfig;

const SOURCE_NAME: &str = "prometheus_remote_write";

fn default_address() -> SocketAddr {
    "0.0.0.0:9090".parse().unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrometheusRemoteWriteConfig {
    address: SocketAddr,
    tls: Option<TlsConfig>,
    auth: Option<HttpSourceAuthConfig>,

    acknowledgements: bool,
}

impl GenerateConfig for PrometheusRemoteWriteConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            address: default_address(),
            tls: None,
            auth: None,
            acknowledgements: false,
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<PrometheusRemoteWriteConfig>(SOURCE_NAME)
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SourceConfig for PrometheusRemoteWriteConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let source = RemoteWriteSource;
        let acknowledgements = self.acknowledgements.clone();

        source
            .run(
                self.address,
                Method::POST,
                "/write",
                true,
                &self.tls,
                &self.auth,
                ctx,
                acknowledgements,
            )
            .await
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        SOURCE_NAME
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.address)]
    }
}

#[derive(Clone)]
struct RemoteWriteSource;

impl RemoteWriteSource {
    fn decode_body(&self, body: Bytes) -> Result<Vec<Event>, ErrorMessage> {
        let request = proto::WriteRequest::decode(body).map_err(|err| {
            ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                format!("Could not decode write request: {}", err),
            )
        })?;

        Ok(vec![])
    }
}

impl HttpSource for RemoteWriteSource {
    fn build_events(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        mut body: Bytes,
    ) -> Result<Vec<Event>, ErrorMessage> {
        if headers
            .get("Content-Encoding")
            .map(|header| header.as_ref())
            != Some(&b"snappy"[..])
        {
            body = decode(&Some("snappy".to_string()), body)?;
        }

        self.decode_body(body)
    }
}
