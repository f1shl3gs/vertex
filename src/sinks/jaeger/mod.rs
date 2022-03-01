mod grpc;
mod http;
mod udp;

use self::http::HttpSinkConfig;
use async_trait::async_trait;
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription};
use framework::sink::util::udp::UdpSinkConfig;
use framework::{Healthcheck, Sink};
use jaeger::agent::{serialize_batch, BufferClient};
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
        match &self.mode {
            Mode::Udp(config) => {
                config.build(cx, move |event| {
                    // TODO: This buffer_client is dummy, rework it in the future
                    let mut buffer_client = BufferClient::default();
                    let trace = event.into_trace();

                    match serialize_batch(
                        &mut buffer_client,
                        trace.into(),
                        jaeger::agent::UDP_PACKET_MAX_LENGTH,
                    ) {
                        Ok(data) => Some(data.into()),
                        Err(err) => {
                            warn!(message = "Encode batch failed", ?err);

                            None
                        }
                    }
                })
            }

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
