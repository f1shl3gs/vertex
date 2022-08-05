use codecs::encoding::{Framer, FramingConfig, SerializerConfig, SinkType};
use codecs::{Encoder, EncodingConfigWithFraming};
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription};
#[cfg(unix)]
use framework::sink::util::unix::UnixSinkConfig;
use framework::sink::util::{tcp::TcpSinkConfig, udp::UdpSinkConfig};
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
// TODO: add back when serde-rs/serde#1358 is addressed
// #[serde(deny_unknown_fields)]
pub struct SocketSinkConfig {
    #[serde(flatten)]
    pub mode: Mode,
    pub encoding: EncodingConfigWithFraming,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Tcp(TcpSinkConfig),
    Udp(UdpSinkConfig),
    #[cfg(unix)]
    Unix(UnixSinkConfig),
}

inventory::submit! {
    SinkDescription::new::<SocketSinkConfig>("socket")
}

impl GenerateConfig for SocketSinkConfig {
    fn generate_config() -> String {
        r#"
# The type of socket to use.
#
# Avaiable values:
#   tcp: TCP socket
#   udp: UDP socket
#   unix: Unix domain socket (Linux only)
mode: tcp

# The unix socket path. This should be the absolute path.
# path: /path/to/socket

# The address to connect to. The address must include a port.
address: 127.0.0.1:5000

# Configures the encoding specific sink behavior
encoding:
  # The encoding codec used to serialize the events before
  # outputting.
  #
  # Avaiable values:
  #   json: JSON encoded event
  #   text: The message field from the event.
  codec: json

  # Prevent the sink from encoding the specified fields
  # except_fields:
  # - foo
  # - foo.bar

  # Makes the sink encode only the specified fields
  # only_fields: foo

  # How to format event timestamps
  #
  # Avaiable values:
  #   rfc3339: Formats as a RFC3339 string -- default
  #   unix:    Formats as a unix timestamp

"#
        .into()
    }
}

impl SocketSinkConfig {
    // TODO: add ack support
    pub const fn new(mode: Mode, encoding: EncodingConfigWithFraming) -> Self {
        SocketSinkConfig { mode, encoding }
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
impl SinkConfig for SocketSinkConfig {
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

    fn sink_type(&self) -> &'static str {
        "socket"
    }
}
