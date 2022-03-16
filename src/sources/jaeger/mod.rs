mod grpc;
mod http;
mod udp;

use std::net::SocketAddr;

use async_trait::async_trait;
use framework::config::{
    DataType, GenerateConfig, Output, Resource, SourceConfig, SourceContext, SourceDescription,
};
use framework::tls::TlsConfig;
use framework::Source;
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};

// See https://www.jaegertracing.io/docs/1.31/getting-started/
const PROTOCOL_COMPACT_THRIFT_OVER_UDP_PORT: u16 = 6831;
const PROTOCOL_BINARY_THRIFT_OVER_UDP_PORT: u16 = 6832;

const fn default_max_packet_size() -> usize {
    65000
}

fn default_thrift_compact_socketaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), PROTOCOL_COMPACT_THRIFT_OVER_UDP_PORT)
}

fn default_thrift_binary_socketaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), PROTOCOL_BINARY_THRIFT_OVER_UDP_PORT)
}

fn default_thrift_http_endpoint() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 14268)
}

fn default_grpc_endpoint() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 14250)
}

/// The Agent can only receive spans over UDP in Thrift format.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-udp-stable
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThriftCompactConfig {
    #[serde(default = "default_thrift_compact_socketaddr")]
    endpoint: SocketAddr,
    #[serde(default = "default_max_packet_size")]
    max_packet_size: usize,
    #[serde(default)]
    socket_buffer_size: Option<usize>,
}

/// Most Jaeger Clients use Thrift’s compact encoding, however some client libraries
/// do not support it (notably, Node.js) and use Thrift’s binary encoding.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-udp-stable
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ThriftBinaryConfig {
    #[serde(default = "default_thrift_binary_socketaddr")]
    endpoint: SocketAddr,
    #[serde(default = "default_max_packet_size")]
    max_packet_size: usize,
    #[serde(default)]
    socket_buffer_size: Option<usize>,
}

/// In some cases it is not feasible to deploy Jaeger Agent next to the application,
/// for example, when the application code is running as AWS Lambda function.
/// In these scenarios the Jaeger Clients can be configured to submit spans directly
/// to the Collectors over HTTP/HTTPS.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThriftHttpConfig {
    #[serde(default = "default_thrift_http_endpoint")]
    endpoint: SocketAddr,
    #[serde(default)]
    tls: Option<TlsConfig>,
}

/// In a typical Jaeger deployment, Agents receive spans from Clients and forward them to Collectors
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#protobuf-via-grpc-stable
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcServerConfig {
    #[serde(default = "default_grpc_endpoint")]
    endpoint: SocketAddr,
}

/// There a lot APIs for receiving spans
///
/// See https://www.jaegertracing.io/docs/1.31/apis/
#[derive(Debug, Deserialize, Serialize)]
struct Protocols {
    thrift_http: Option<ThriftHttpConfig>,
    thrift_compact: Option<ThriftCompactConfig>,
    thrift_binary: Option<ThriftBinaryConfig>,
    grpc: Option<GrpcServerConfig>,
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
        let shutdown = cx.shutdown.shared();
        let source = cx.key.to_string();
        let mut handles = vec![];

        if let Some(config) = &self.protocols.thrift_compact {
            handles.push(tokio::spawn(udp::serve(
                source.clone(),
                config.endpoint,
                config.max_packet_size,
                config.socket_buffer_size,
                shutdown.clone(),
                |data| match jaeger::agent::deserialize_compact_batch(data) {
                    Ok(batch) => Ok(batch),
                    Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err)),
                },
                cx.output.clone(),
            )));
        }

        if let Some(config) = &self.protocols.thrift_binary {
            handles.push(tokio::spawn(udp::serve(
                source,
                config.endpoint,
                config.max_packet_size,
                config.socket_buffer_size,
                shutdown.clone(),
                |data| match jaeger::agent::deserialize_binary_batch(data) {
                    Ok(batch) => Ok(batch),
                    Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err)),
                },
                cx.output.clone(),
            )));
        }

        if let Some(config) = &self.protocols.grpc {
            handles.push(tokio::spawn(grpc::serve(
                config.clone(),
                shutdown.clone(),
                cx.output.clone(),
            )));
        }

        if let Some(config) = &self.protocols.thrift_http {
            handles.push(tokio::spawn(http::serve(
                config.clone(),
                shutdown,
                cx.output,
            )));
        }

        Ok(Box::pin(async move {
            // TODO: we need something like `errgroup` in Golang
            let _result = futures::future::join_all(handles).await;

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }

    fn source_type(&self) -> &'static str {
        "jaeger"
    }

    fn resources(&self) -> Vec<Resource> {
        let mut resources = vec![];

        if let Some(config) = &self.protocols.thrift_http {
            resources.push(Resource::tcp(config.endpoint));
        }

        if let Some(config) = &self.protocols.thrift_compact {
            resources.push(Resource::udp(config.endpoint));
        }

        if let Some(config) = &self.protocols.thrift_binary {
            resources.push(Resource::udp(config.endpoint))
        }

        if let Some(config) = &self.protocols.grpc {
            resources.push(Resource::tcp(config.endpoint))
        }

        resources
    }
}
