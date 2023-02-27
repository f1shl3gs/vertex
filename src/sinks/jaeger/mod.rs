mod grpc;
mod http;
mod serializer;
mod udp;

use async_trait::async_trait;
use codecs::encoding::{Serializer, Transformer};
use codecs::Encoder;
use configurable::configurable_component;
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::sink::util::udp::UdpSinkConfig;
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

use self::http::HttpSinkConfig;
use self::serializer::ThriftSerializer;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CollectorConfig {}

#[configurable_component(sink, name = "jaeger")]
#[derive(Debug)]
#[serde(rename_all = "lowercase", tag = "protocol")]
enum JaegerConfig {
    Udp(UdpSinkConfig),

    Http(HttpSinkConfig),
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SinkConfig for JaegerConfig {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let transformer = Transformer::default();
        let encoder = Encoder::<()>::new(Serializer::Boxed(Box::new(ThriftSerializer::new())));

        match &self {
            JaegerConfig::Udp(config) => config.build(transformer, encoder),

            JaegerConfig::Http(config) => config.build(cx.proxy),
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<JaegerConfig>()
    }
}
