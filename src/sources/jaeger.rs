use std::net::SocketAddr;

use async_trait::async_trait;
use framework::config::{DataType, Output, Resource, SourceConfig, SourceContext};
use framework::Source;
use serde::{Deserialize, Serialize};
use framework::tls::TlsConfig;

const fn default_udp_sockaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 6831)
}

const fn default_thrift_http_endpoint() -> SocketAddr {
    SocketAddr::new([127, 0, 0, 1].into(), 14268)
}

#[derive(Debug, Deserialize, Serialize)]
struct UdpConfig {
    #[serde(default = "default_udp_sockaddr")]
    address: SocketAddr,

    max_packet_size: usize,

    socket_buffer_size: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpServerConfig {
    #[serde(default = "default_thrift_http_endpoint")]
    endpoint: SocketAddr,
    #[serde(default)]
    tls: Option<TlsConfig>,
}

struct Protocols {
    thrift_http: Option<HttpServerConfig>,
    thrift_compact: Option<UdpConfig>,
    thrift_binary: Option<UdpConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CollectorConfig {
    protocols: Protocols,
}

#[derive(Debug, Deserialize, Serialize)]
struct JaegerConfig {
    pub agent: UdpConfig,
    pub collector: CollectorConfig,
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SourceConfig for JaegerConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }

    fn source_type(&self) -> &'static str {
        "jaeger"
    }

    fn resources(&self) -> Vec<Resource> {
        todo!()
    }
}
