mod grpc;
mod http;
mod udp;

use async_trait::async_trait;
use configurable::configurable_component;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::Source;
use futures_util::stream::{FuturesUnordered, StreamExt};

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
        let mut tasks = FuturesUnordered::new();

        if let Some(config) = &self.thrift_compact {
            tasks.push(tokio::spawn(udp::serve(
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
            tasks.push(tokio::spawn(udp::serve(
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
            tasks.push(tokio::spawn(grpc::serve(
                config.clone(),
                shutdown.clone(),
                cx.output.clone(),
            )));
        }

        if let Some(config) = &self.thrift_http {
            tasks.push(tokio::spawn(http::serve(
                config.clone(),
                shutdown,
                cx.output,
            )));
        }

        if tasks.is_empty() {
            return Err("At least one API should be enabled".into());
        }

        Ok(Box::pin(async move {
            while let Some(result) = tasks.next().await {
                match result {
                    Ok(Ok(_)) => {}
                    Ok(Err(err)) => {
                        error!(message = "jaeger serve failed", err);
                        return Err(());
                    }
                    Err(err) => {
                        error!(message = "spawn jaeger server failed", %err);
                        return Err(());
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::traces()]
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
        crate::testing::generate_config::<Config>()
    }
}
