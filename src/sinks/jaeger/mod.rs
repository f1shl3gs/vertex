mod grpc;
mod http;
mod serializer;
mod udp;

use self::http::HttpSinkConfig;
use crate::sinks::jaeger::serializer::ThriftSerializer;
use async_trait::async_trait;
use codecs::encoding::{Serializer, Transformer};
use codecs::Encoder;
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription};
use framework::sink::util::udp::UdpSinkConfig;
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CollectorConfig {}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "protocol", rename_all = "snake_case")]
enum Mode {
    Udp(UdpSinkConfig),
    Http(HttpSinkConfig),
}

#[derive(Debug, Deserialize, Serialize)]
struct JaegerConfig {
    #[serde(flatten)]
    pub mode: Mode,
}

impl GenerateConfig for JaegerConfig {
    fn generate_config() -> String {
        r#"
# The type jaeger compoent
#
protocol: udp

# The address to connect to. The address must include a port.
address: 127.0.0.1:6831

"#
        .into()
    }
}

inventory::submit! {
    SinkDescription::new::<JaegerConfig>("jaeger")
}

#[async_trait]
#[typetag::serde(name = "jaeger")]
impl SinkConfig for JaegerConfig {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let transformer = Transformer::default();
        let encoder = Encoder::<()>::new(Serializer::Boxed(Box::new(ThriftSerializer::new())));

        match &self.mode {
            Mode::Udp(config) => config.build(transformer, encoder),

            Mode::Http(config) => config.build(cx.proxy, cx.acker),
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Trace
    }

    fn sink_type(&self) -> &'static str {
        "jaeger"
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
