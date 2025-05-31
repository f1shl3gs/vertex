use codecs::encoding::{Framer, FramingConfig, SerializerConfig, SinkType};
use codecs::{Encoder, EncodingConfig, EncodingConfigWithFraming};
use configurable::{Configurable, configurable_component};
use framework::config::{DataType, SinkConfig, SinkContext};
#[cfg(unix)]
use framework::sink::util::unix::UnixSinkConfig;
use framework::sink::util::{tcp::TcpSinkConfig, udp::UdpSinkConfig};
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    /// Listen on TCP.
    Tcp {
        #[serde(flatten)]
        config: TcpSinkConfig,

        #[serde(flatten)]
        encoding: EncodingConfigWithFraming,
    },

    /// Listen on UDP.
    Udp {
        #[serde(flatten)]
        config: UdpSinkConfig,

        encoding: EncodingConfig,
    },

    /// Listen on a Unix domain socket (UDS), in stream mode.
    #[cfg(unix)]
    Unix {
        #[serde(flatten)]
        config: UnixSinkConfig,

        #[serde(flatten)]
        encoding: EncodingConfigWithFraming,
    },
}

#[configurable_component(sink, name = "socket")]
pub struct Config {
    #[serde(flatten)]
    pub mode: Mode,

    #[serde(default)]
    pub acknowledgements: bool,
}

impl Config {
    // TODO: add ack support
    pub const fn new(mode: Mode) -> Self {
        Config {
            mode,
            acknowledgements: false,
        }
    }

    pub fn make_basic_tcp_config(address: String) -> Self {
        Self::new(Mode::Tcp {
            config: TcpSinkConfig::from_address(address),
            encoding: (None::<FramingConfig>, SerializerConfig::Text).into(),
        })
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "socket")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        match &self.mode {
            Mode::Tcp { config, encoding } => {
                let transformer = encoding.transformer();
                let (framer, serializer) = encoding.build(SinkType::MessageBased);
                let encoder = Encoder::<Framer>::new(framer, serializer);

                config.build(transformer, encoder)
            }
            Mode::Udp { config, encoding } => {
                let transformer = encoding.transformer();
                let serializer = encoding.build();
                let encoder = Encoder::<()>::new(serializer);
                config.build(transformer, encoder)
            }
            #[cfg(unix)]
            Mode::Unix { config, encoding } => {
                let transformer = encoding.transformer();
                let (framer, serializer) = encoding.build(SinkType::MessageBased);
                let encoder = Encoder::<Framer>::new(framer, serializer);

                config.build(transformer, encoder)
            }
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
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
