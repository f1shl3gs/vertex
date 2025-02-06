pub mod tcp;
pub mod udp;
#[cfg(unix)]
mod unix;

use codecs::DecodingConfig;
use configurable::{configurable_component, Configurable};
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::Source;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
enum Mode {
    /// Listen on TCP
    Tcp(tcp::Config),
    Udp(udp::Config),
    #[cfg(unix)]
    UnixDatagram(unix::Config),
    #[cfg(unix)]
    UnixStream(unix::Config),
}

#[configurable_component(source, name = "socket")]
pub struct Config {
    #[serde(flatten)]
    mode: Mode,
}

impl Config {
    pub fn simple_tcp(addr: SocketAddr) -> Self {
        Config {
            mode: Mode::Tcp(tcp::Config::simple(addr)),
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "socket")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        match &self.mode {
            Mode::Tcp(config) => config.run(cx),
            Mode::Udp(config) => config.run(cx),
            #[cfg(unix)]
            Mode::UnixDatagram(config) => {
                let decoding = config.decoding.clone();
                let framing = config
                    .framing
                    .clone()
                    .unwrap_or_else(|| decoding.default_message_based_framing());
                let decoder = DecodingConfig::new(framing, decoding).build()?;

                config.run_datagram(decoder, cx)
            }
            #[cfg(unix)]
            Mode::UnixStream(config) => {
                let decoding = config.decoding.clone();
                let framing = config
                    .framing
                    .clone()
                    .unwrap_or_else(|| decoding.default_stream_framing());
                let decoder = DecodingConfig::new(framing, decoding).build()?;

                config.run_stream(decoder, cx)
            }
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        match &self.mode {
            Mode::Tcp(config) => {
                vec![config.resource()]
            }
            Mode::Udp(config) => {
                vec![config.resource()]
            }
            Mode::UnixDatagram(config) | Mode::UnixStream(config) => {
                vec![config.resource()]
            }
        }
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
