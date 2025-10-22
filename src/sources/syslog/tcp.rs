use std::net::SocketAddr;
use std::time::Duration;

use codecs::Decoder;
use codecs::decoding::{DecodeError, OctetCountingDecoder, SyslogDeserializer};
use configurable::Configurable;
use event::Events;
use framework::Source;
use framework::config::{Resource, SourceContext};
use framework::source::tcp::{SocketListenAddr, TcpNullAcker, TcpSource};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::TlsConfig;
use serde::{Deserialize, Serialize};
use value::OwnedValuePath;

use super::handle_events;

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The address to listen for connections on, or systemd#N to use the Nth
    /// socket passed by systemd socket activation. If an address is used it
    /// must include a port.
    #[configurable(format = "ip-address", example = "0.0.0.0:9000")]
    listen: SocketListenAddr,

    /// Configures the TCP keepalive behavior for the connection to the source.
    keepalive: Option<TcpKeepaliveConfig>,

    /// Configures the TLS options for incoming connections.
    tls: Option<TlsConfig>,

    /// Configures the recive buffer size using the "SO_RCVBUF" option on the socket.
    #[serde(default, with = "humanize::bytes::serde_option")]
    pub receive_buffer_bytes: Option<usize>,

    /// The max number of TCP connections that will be processed.
    connection_limit: Option<usize>,
}

impl Config {
    pub fn resource(&self) -> Resource {
        match self.listen {
            SocketListenAddr::SocketAddr(addr) => Resource::tcp(addr),
            SocketListenAddr::SystemFd(fd) => Resource::SystemFd(fd),
        }
    }

    pub fn build(
        &self,
        cx: SourceContext,
        max_length: usize,
        host_key: OwnedValuePath,
    ) -> crate::Result<Source> {
        let source = SyslogTcpSource {
            max_length,
            host_key,
        };
        let shutdown_timeout = Duration::from_secs(30);

        source.run(
            self.listen,
            self.keepalive,
            shutdown_timeout,
            self.tls.as_ref(),
            self.receive_buffer_bytes,
            cx,
            self.connection_limit,
        )
    }
}

#[derive(Clone, Debug)]
struct SyslogTcpSource {
    max_length: usize,
    host_key: OwnedValuePath,
}

impl TcpSource for SyslogTcpSource {
    type Error = DecodeError;
    type Item = Events;
    type Decoder = Decoder;
    type Acker = TcpNullAcker;

    fn decoder(&self) -> Self::Decoder {
        Decoder::new(
            OctetCountingDecoder::new_with_max_length(self.max_length).into(),
            SyslogDeserializer::default().into(),
        )
    }

    fn handle_events(&self, batch: &mut [Events], peer: SocketAddr, _size: usize) {
        let default_host = Some(peer);

        for events in batch {
            handle_events(events, &self.host_key, default_host)
        }
    }

    fn build_acker(&self, _item: &[Self::Item]) -> Self::Acker {
        TcpNullAcker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_tcp_with_receive_buffer_size() {
        let config: Config =
            serde_yaml::from_str("address: 127.0.0.1:12345\nreceive_buffer_bytes: 1ki").unwrap();

        assert_eq!(config.receive_buffer_bytes, Some(1024usize));
    }

    #[test]
    fn config_tcp_with_keepalive() {
        let config: Config =
            serde_yaml::from_str("address: 127.0.0.1:12345\nkeepalive:\n  timeout: 120s").unwrap();

        assert_eq!(
            config.keepalive.unwrap().timeout,
            Some(Duration::from_secs(120))
        );
    }
}
