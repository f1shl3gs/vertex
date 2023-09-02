use codecs::encoding::{Framer, FramingConfig, SerializerConfig, SinkType};
use codecs::{Encoder, EncodingConfigWithFraming};
use configurable::{configurable_component, Configurable};
use framework::config::{DataType, SinkConfig, SinkContext};
#[cfg(unix)]
use framework::sink::util::unix::UnixSinkConfig;
use framework::sink::util::{tcp::TcpSinkConfig, udp::UdpSinkConfig};
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

#[configurable_component(sink, name = "socket")]
pub struct Config {
    #[serde(flatten)]
    pub mode: Mode,

    pub encoding: EncodingConfigWithFraming,
}

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    /// Listen on TCP.
    Tcp(TcpSinkConfig),

    /// Listen on UDP.
    Udp(UdpSinkConfig),

    /// Listen on a Unix domain socket (UDS), in stream mode.
    #[cfg(unix)]
    Unix(UnixSinkConfig),
}

impl Config {
    // TODO: add ack support
    pub const fn new(mode: Mode, encoding: EncodingConfigWithFraming) -> Self {
        Config { mode, encoding }
    }

    pub fn make_basic_tcp_config(address: String) -> Self {
        Self::new(
            Mode::Tcp(TcpSinkConfig::from_address(address)),
            (None::<FramingConfig>, SerializerConfig::Text).into(),
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "socket")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let transformer = self.encoding.transformer();
        let (framer, serializer) = self.encoding.build(SinkType::MessageBased);
        let encoder = Encoder::<Framer>::new(framer, serializer);

        match &self.mode {
            Mode::Tcp(config) => config.build(transformer, encoder),
            Mode::Udp(config) => config.build(transformer, encoder),
            #[cfg(unix)]
            Mode::Unix(config) => config.build(transformer, encoder),
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Log
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
