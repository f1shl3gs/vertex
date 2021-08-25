use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use crate::{
    sinks::Sink,
    config::{SinkConfig, SinkContext, DataType, Resource},
    tls::TLSConfig,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrometheusExporterConfig {
    pub namespace: Option<String>,

    pub tls: Option<TLSConfig>,

    #[serde(default = "default_listen_address")]
    pub listen: SocketAddr,

    #[serde(default = "default_telemetry_path")]
    pub telemetry_path: String,

    #[serde(default = "default_flush_period", deserialize_with = "crate::config::deserialize_duration", serialize_with = "crate::config::serialize_duration")]
    pub flush_period: chrono::Duration,
}

impl Default for PrometheusExporterConfig {
    fn default() -> Self {
        Self {
            namespace: None,
            tls: None,
            flush_period: default_flush_period(),
            listen: default_listen_address(),
            telemetry_path: default_telemetry_path(),
        }
    }
}

fn default_flush_period() -> chrono::Duration {
    chrono::Duration::seconds(60)
}

fn default_listen_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9100)
}

fn default_telemetry_path() -> String {
    "/metrics".into()
}

#[async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for PrometheusExporterConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<Sink> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn sink_type(&self) -> &'static str {
        "prometheus_exporter"
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }
}

