mod udp;

use std::net::SocketAddr;

use async_trait::async_trait;
use framework::config::{
    DataType, GenerateConfig, Output, Resource, SourceConfig, SourceContext, SourceDescription,
};
use framework::tls::TlsConfig;
use framework::Source;
use serde::{Deserialize, Serialize};

// See https://www.jaegertracing.io/docs/1.31/getting-started/
const PROTOCOL_COMPACT_THRIFT_OVER_UDP_PORT: u16 = 6831;
const PROTOCOL_BINARY_THRIFT_OVER_UDP_PORT: u16 = 6832;

const fn default_max_packet_size() -> usize {
    65000
}

fn default_compact_thrift_sockaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 6831)
}

fn default_thrift_http_endpoint() -> SocketAddr {
    SocketAddr::new([127, 0, 0, 1].into(), 14268)
}

#[derive(Debug, Deserialize, Serialize)]
struct UdpConfig {
    #[serde(default)]
    endpoint: Option<SocketAddr>,
    max_packet_size: usize,
    #[serde(default)]
    socket_buffer_size: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpServerConfig {
    #[serde(default = "default_thrift_http_endpoint")]
    endpoint: SocketAddr,
    #[serde(default)]
    tls: Option<TlsConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Protocols {
    thrift_http: Option<HttpServerConfig>,
    thrift_compact: Option<UdpConfig>,
    thrift_binary: Option<UdpConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JaegerConfig {
    protocols: Protocols,
}

impl GenerateConfig for JaegerConfig {
    fn generate_config() -> String {
        r#""#.into()
    }
}

inventory::submit! {
    SourceDescription::new::<JaegerConfig>("jaeger")
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SourceConfig for JaegerConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        if let Some(ref config) = self.protocols.thrift_compact {
            let max_packet_size = if config.max_packet_size == 0 {
                default_max_packet_size()
            } else {
                config.max_packet_size
            };

            let endpoint = config
                .endpoint
                .unwrap_or_else(default_compact_thrift_sockaddr);

            Ok(udp::source(
                cx.key.to_string(),
                endpoint,
                max_packet_size,
                config.socket_buffer_size,
                cx.shutdown,
                |data| match jaeger::agent::deserialize_compact_batch(data) {
                    Ok(batch) => Ok(batch),
                    Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err)),
                },
                cx.output,
            ))
        } else {
            panic!()
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }

    fn source_type(&self) -> &'static str {
        "jaeger"
    }

    fn resources(&self) -> Vec<Resource> {
        // TODO
        vec![]
    }
}
