use std::net::SocketAddr;
use std::time::Duration;

use codecs::decoding::{DecodeError, DeserializerConfig, FramingConfig};
use codecs::{Decoder, DecodingConfig};
use configurable::Configurable;
use event::Events;
use framework::Source;
use framework::config::{Resource, SourceContext};
use framework::source::tcp::{SocketListenAddr, TcpNullAcker, TcpSource};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::TlsConfig;
use serde::{Deserialize, Serialize};
use value::path::PathPrefix;
use value::{OwnedValuePath, value};

use crate::sources::default_decoding;

const fn default_shutdown_timeout() -> Duration {
    Duration::from_secs(30)
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    address: SocketAddr,

    keepalive: Option<TcpKeepaliveConfig>,

    /// The timeout before a connection is forcefully closed during shutdown.
    #[serde(
        with = "humanize::duration::serde",
        default = "default_shutdown_timeout"
    )]
    shutdown_timeout: Duration,

    /// Overrides the name of the log field used to add the peer host to each
    /// event.
    ///
    /// The value will be the peer host's address, include the port i.e. `1.2.3.4:80`
    host_key: Option<OwnedValuePath>,

    /// Overrides the name of the log field used to add the peer host's port
    /// to each event.
    ///
    /// The value will be the peer host's port i.e. `9000`
    port_key: Option<OwnedValuePath>,

    tls: Option<TlsConfig>,

    /// The size of the receive buffer used for each connection.
    #[serde(with = "humanize::bytes::serde_option")]
    receive_buffer_bytes: Option<usize>,

    /// The maximum number of TCP connections that are allowed at any given time.
    connection_limit: Option<usize>,

    #[serde(default)]
    framing: Option<FramingConfig>,

    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,
}

impl Config {
    pub fn simple(address: SocketAddr) -> Self {
        Self {
            address,
            keepalive: None,
            shutdown_timeout: Duration::from_secs(10),
            host_key: None,
            port_key: None,
            tls: None,
            receive_buffer_bytes: None,
            connection_limit: None,
            framing: None,
            decoding: Default::default(),
        }
    }

    pub fn resource(&self) -> Resource {
        Resource::tcp(self.address)
    }

    pub fn run(&self, cx: SourceContext) -> crate::Result<Source> {
        let decoding = self.decoding.clone();
        let decoder = DecodingConfig::new(
            self.framing
                .clone()
                .unwrap_or_else(|| decoding.default_stream_framing()),
            decoding,
        )
        .build()?;

        let source = RawTcpSource::new(decoder, self.port_key.clone());

        source.run(
            SocketListenAddr::SocketAddr(self.address),
            self.keepalive,
            self.shutdown_timeout,
            self.tls.as_ref(),
            self.receive_buffer_bytes,
            cx,
            self.connection_limit,
        )
    }
}

#[derive(Clone)]
struct RawTcpSource {
    decoder: Decoder,
    port_key: Option<OwnedValuePath>,
}

impl RawTcpSource {
    fn new(decoder: Decoder, port_key: Option<OwnedValuePath>) -> Self {
        RawTcpSource { decoder, port_key }
    }
}

impl TcpSource for RawTcpSource {
    type Error = DecodeError;
    type Item = Events;
    type Decoder = Decoder;
    type Acker = TcpNullAcker;

    fn decoder(&self) -> Self::Decoder {
        self.decoder.clone()
    }

    fn handle_events(&self, batch: &mut [Events], peer: SocketAddr, _size: usize) {
        for events in batch {
            events.for_each_log(|log| {
                let host = peer.ip().to_string();
                let port = peer.port();

                log.metadata_mut().value_mut().insert(
                    "socket",
                    value!({
                        "host": host,
                        "port": port,
                    }),
                );

                if let Some(path) = self.port_key.as_ref() {
                    log.insert((PathPrefix::Event, path), port);
                }
            });
        }
    }

    fn build_acker(&self, _item: &[Self::Item]) -> Self::Acker {
        TcpNullAcker
    }
}
