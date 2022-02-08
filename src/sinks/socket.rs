use event::encoding::EncodingConfig;
use framework::config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription};
use framework::{Healthcheck, Sink};
use serde::{Deserialize, Serialize};

#[cfg(unix)]
use framework::sink::util::unix::UnixSinkConfig;
use framework::sink::util::{encode_log, tcp::TcpSinkConfig, udp::UdpSinkConfig, Encoding};

#[derive(Deserialize, Serialize, Debug)]
// TODO: add back when serde-rs/serde#1358 is addressed
// #[serde(deny_unknown_fields)]
pub struct SocketSinkConfig {
    #[serde(flatten)]
    pub mode: Mode,
    pub encoding: EncodingConfig<Encoding>,
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
    pub const fn new(mode: Mode, encoding: EncodingConfig<Encoding>) -> Self {
        SocketSinkConfig { mode, encoding }
    }

    pub fn make_basic_tcp_config(address: String) -> Self {
        Self::new(
            Mode::Tcp(TcpSinkConfig::from_address(address)),
            EncodingConfig::from(Encoding::Text),
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "socket")]
impl SinkConfig for SocketSinkConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let encoding = self.encoding.clone();
        let encode_event = move |event| encode_log(event, &encoding);
        match &self.mode {
            Mode::Tcp(config) => config.build(cx, encode_event),
            Mode::Udp(config) => config.build(cx, encode_event),
            #[cfg(unix)]
            Mode::Unix(config) => config.build(cx, encode_event),
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "socket"
    }
}
