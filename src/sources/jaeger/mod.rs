mod grpc;
mod http;
mod udp;

use async_trait::async_trait;
use configurable::configurable_component;
use framework::config::{DataType, Output, Resource, SourceConfig, SourceContext};
use framework::Source;

/// Jaeger components implement various APIs for saving or retrieving trace data.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/
#[configurable_component(source, name = "jaeger")]
struct Config {
    thrift_http: Option<http::ThriftHttpConfig>,
    thrift_compact: Option<udp::ThriftCompactConfig>,
    thrift_binary: Option<udp::ThriftBinaryConfig>,
    grpc: Option<grpc::GrpcServerConfig>,
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let shutdown = cx.shutdown;
        let source = cx.key.to_string();
        let mut handles = vec![];

        if let Some(config) = &self.thrift_compact {
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

        if let Some(config) = &self.thrift_binary {
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

        if let Some(config) = &self.grpc {
            handles.push(tokio::spawn(grpc::serve(
                config.clone(),
                shutdown.clone(),
                cx.output.clone(),
            )));
        }

        if let Some(config) = &self.thrift_http {
            handles.push(tokio::spawn(http::serve(
                config.clone(),
                shutdown,
                cx.output,
            )));
        }

        if handles.is_empty() {
            return Err("At least one API should be enabled".into());
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

    fn resources(&self) -> Vec<Resource> {
        let mut resources = vec![];

        if let Some(config) = &self.thrift_http {
            resources.push(Resource::tcp(config.endpoint));
        }

        if let Some(config) = &self.thrift_compact {
            resources.push(Resource::udp(config.endpoint));
        }

        if let Some(config) = &self.thrift_binary {
            resources.push(Resource::udp(config.endpoint))
        }

        if let Some(config) = &self.grpc {
            resources.push(Resource::tcp(config.endpoint))
        }

        resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
