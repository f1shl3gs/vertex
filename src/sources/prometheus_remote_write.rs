use crate::config::{
    DataType, GenerateConfig, Output, Resource, SourceConfig, SourceContext, SourceDescription,
};
use crate::sources::Source;
use crate::tls::TlsConfig;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::net::SocketAddr;

const SOURCE_NAME: &str = "prometheus_remote_write";

const fn default_address() -> SocketAddr {
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
        todo!()
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
