mod encoder;
mod grpc;
mod http;
mod udp;

use async_trait::async_trait;
use codecs::encoding::Transformer;
use configurable::configurable_component;
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::sink::util::udp::UdpSinkConfig;
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

use self::encoder::ThriftEncoder;
use self::http::HttpSinkConfig;

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

        match &self {
            JaegerConfig::Udp(config) => config.build(transformer, ThriftEncoder::new()),

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
