mod encoder;
mod grpc;
mod http;

use codecs::encoding::Transformer;
use configurable::configurable_component;
use framework::config::{InputType, SinkConfig, SinkContext};
use framework::sink::net::UdpSinkConfig;
use framework::{Healthcheck, Sink};

use self::encoder::ThriftEncoder;
use self::http::HttpSinkConfig;

#[allow(clippy::large_enum_variant)]
#[configurable_component(sink, name = "jaeger")]
#[serde(rename_all = "lowercase", tag = "protocol")]
enum Config {
    Udp(UdpSinkConfig),

    Http(HttpSinkConfig),
}

#[async_trait::async_trait]
#[typetag::serde(name = "jaeger")]
impl SinkConfig for Config {
    async fn build(&self, cx: SinkContext) -> framework::Result<(Sink, Healthcheck)> {
        let transformer = Transformer::default();

        match &self {
            Config::Udp(config) => config.build(transformer, ThriftEncoder::default()),

            Config::Http(config) => config.build(cx.proxy),
        }
    }

    fn input_type(&self) -> InputType {
        InputType::trace()
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
