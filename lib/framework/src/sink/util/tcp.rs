use std::{
    io::ErrorKind,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use async_trait::async_trait;
use backoff::ExponentialBackoff;
use bytes::{Bytes, BytesMut};
use codecs::encoding::Transformer;
use configurable::Configurable;
use event::{Event, EventContainer, Events};
use futures::{stream::BoxStream, task::noop_waker_ref, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, ReadBuf},
    net::TcpStream,
    time::sleep,
};
use tokio_util::codec::Encoder;

use super::{SinkBuildError, SocketMode};
use crate::batch::EncodedEvent;
use crate::dns;
use crate::sink::util::socket_bytes_sink::{BytesSink, ShutdownCheck};
use crate::sink::VecSinkExt;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::{MaybeTlsSettings, MaybeTlsStream, TlsConfig, TlsError};
use crate::OpenGauge;
use crate::StreamSink;
use crate::{Healthcheck, Sink};

#[derive(Debug, Error)]
enum TcpError {
    #[error("Connect error: {0}")]
    Connect(TlsError),
    #[error("Unable to resolve DNS: {0}")]
    ResolveDns(dns::DnsError),
    #[error("No addresses returned.")]
    NoAddresses,
}

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TcpSinkConfig {
    /// The address to connect to. The address must include a port.
    #[configurable(required)]
    address: String,

    keepalive: Option<TcpKeepaliveConfig>,
    tls: Option<TlsConfig>,
    send_buffer_bytes: Option<usize>,
}

impl TcpSinkConfig {
    pub const fn new(
        address: String,
        keepalive: Option<TcpKeepaliveConfig>,
        tls: Option<TlsConfig>,
        send_buffer_bytes: Option<usize>,
    ) -> Self {
        Self {
            address,
            keepalive,
            tls,
            send_buffer_bytes,
        }
    }

    pub const fn from_address(address: String) -> Self {
        Self {
            address,
            keepalive: None,
            tls: None,
            send_buffer_bytes: None,
        }
    }

    pub fn build(
        &self,
        transformer: Transformer,
        encoder: impl Encoder<Event, Error = codecs::encoding::EncodingError>
            + Clone
            + Send
            + Sync
            + 'static,
    ) -> crate::Result<(Sink, Healthcheck)> {
        let uri = self.address.parse::<http::Uri>()?;
        let host = uri.host().ok_or(SinkBuildError::MissingHost)?.to_string();
        let port = uri.port_u16().ok_or(SinkBuildError::MissingPort)?;
        let tls = MaybeTlsSettings::from_config(&self.tls, false)?;
        let connector = TcpConnector::new(host, port, self.keepalive, tls, self.send_buffer_bytes);
        let sink = TcpSink::new(connector.clone(), transformer, encoder);

        Ok((
            Sink::Stream(Box::new(sink)),
            Box::pin(async move { connector.healthcheck().await }),
        ))
    }
}

#[derive(Clone)]
struct TcpConnector {
    host: String,
    port: u16,
    keepalive: Option<TcpKeepaliveConfig>,
    tls: MaybeTlsSettings,
    send_buffer_bytes: Option<usize>,
}

impl TcpConnector {
    const fn new(
        host: String,
        port: u16,
        keepalive: Option<TcpKeepaliveConfig>,
        tls: MaybeTlsSettings,
        send_buffer_bytes: Option<usize>,
    ) -> Self {
        Self {
            host,
            port,
            keepalive,
            tls,
            send_buffer_bytes,
        }
    }

    #[cfg(test)]
    fn from_host_port(host: String, port: u16) -> Self {
        Self::new(host, port, None, None.into(), None)
    }

    const fn fresh_backoff() -> ExponentialBackoff {
        // TODO: make configurable
        ExponentialBackoff::from_millis(2)
            .factor(250)
            .max_delay(Duration::from_secs(60))
    }

    async fn connect(&self) -> Result<MaybeTlsStream<TcpStream>, TcpError> {
        let ip = dns::Resolver
            .lookup_ip(self.host.clone())
            .await
            .map_err(TcpError::ResolveDns)?
            .next()
            .ok_or(TcpError::NoAddresses)?;

        let addr = SocketAddr::new(ip, self.port);
        self.tls
            .connect(&self.host, &addr)
            .await
            .map_err(TcpError::Connect)
            .map(|mut maybe_tls| {
                if let Some(keepalive) = self.keepalive {
                    if let Err(err) = maybe_tls.set_keepalive(keepalive) {
                        warn!(message = "Failed configuring TCP keepalive.", %err);
                    }
                }

                if let Some(send_buffer_bytes) = self.send_buffer_bytes {
                    if let Err(err) = maybe_tls.set_send_buffer_bytes(send_buffer_bytes) {
                        warn!(message = "Failed configuring send buffer size on TCP socket.", %err);
                    }
                }

                maybe_tls
            })
    }

    async fn connect_backoff(&self) -> MaybeTlsStream<TcpStream> {
        let mut backoff = Self::fresh_backoff();
        loop {
            match self.connect().await {
                Ok(socket) => {
                    // TODO: metric
                    return socket;
                }
                Err(err) => {
                    // TODO: handle error and metric
                    error!(
                        message = "Unable to connect",
                        %err
                    );

                    sleep(backoff.next().unwrap()).await;
                }
            }
        }
    }

    async fn healthcheck(&self) -> crate::Result<()> {
        self.connect().await.map(|_| ()).map_err(Into::into)
    }
}

struct TcpSink<E>
where
    E: Clone + Send + Sync + Encoder<Event, Error = codecs::encoding::EncodingError>,
{
    connector: TcpConnector,
    transformer: Transformer,
    encoder: E,
}

impl<E> TcpSink<E>
where
    E: Clone + Send + Sync + Encoder<Event, Error = codecs::encoding::EncodingError> + 'static,
{
    fn new(connector: TcpConnector, transformer: Transformer, encoder: E) -> Self {
        Self {
            connector,
            transformer,
            encoder,
        }
    }

    async fn connect(&self) -> BytesSink<MaybeTlsStream<TcpStream>> {
        let stream = self.connector.connect_backoff().await;
        BytesSink::new(stream, Self::shutdown_check, SocketMode::Tcp)
    }

    fn shutdown_check(stream: &mut MaybeTlsStream<TcpStream>) -> ShutdownCheck {
        // Test if the remote has issued a disconnect by calling read(2)
        // with a 1 sized buffer.
        //
        // This can return a proper disconnect error or `Ok(0)`
        // which means the pipe is broken and we should try to reconnect.
        //
        // If this returns `Poll::Pending` we know the connection is still
        // valid and the write will most likely succeed.
        let mut cx = Context::from_waker(noop_waker_ref());
        let mut buf = [0u8; 1];
        let mut buf = ReadBuf::new(&mut buf);
        match Pin::new(stream).poll_read(&mut cx, &mut buf) {
            Poll::Ready(Err(err)) => ShutdownCheck::Error(err),
            Poll::Ready(Ok(())) if buf.filled().is_empty() => {
                // Maybe this is only a sign to close the channel,
                // in which case we should try to flush our buffers
                // before disconnecting.
                ShutdownCheck::Close("ShutdownCheck::Close")
            }
            _ => ShutdownCheck::Alive,
        }
    }
}

#[async_trait]
impl<E> StreamSink for TcpSink<E>
where
    E: Clone + Send + Sync + Encoder<Event, Error = codecs::encoding::EncodingError> + 'static,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        // We need [Peekable](https://docs.rs/futures/0.3.6/futures/stream/struct.Peekable.html)
        // for initiating connection only when we have something to send.
        let mut encoder = self.encoder.clone();
        let mut input = input
            .map(|events| {
                events
                    .into_events()
                    .map(|mut event| {
                        let byte_size = event.size_of();
                        let finalizers = event.metadata_mut().take_finalizers();
                        self.transformer.transform(&mut event);
                        let mut buf = BytesMut::new();
                        if encoder.encode(event, &mut buf).is_ok() {
                            let item = buf.freeze();
                            EncodedEvent {
                                item,
                                finalizers,
                                byte_size,
                            }
                        } else {
                            EncodedEvent::new(Bytes::new(), 0)
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flat_map(futures::stream::iter)
            .peekable();

        while Pin::new(&mut input).peek().await.is_some() {
            let mut sink = self.connect().await;
            let _open_token = OpenGauge::new();

            let result = match sink.send_all_peekable(&mut (&mut input).peekable()).await {
                Ok(()) => sink.close().await,
                Err(err) => Err(err),
            };

            if let Err(err) = result {
                if err.kind() == ErrorKind::Other && err.to_string() == "ShutdownCheck::Close" {
                    debug!(message = "Received EOF from the server, shutdown",);

                    // TODO: metric
                } else {
                    warn!(
                        message = "TCP socket error",
                        %err
                    );

                    // TODO: metric
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use testify::next_addr;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn healthcheck() {
        crate::trace::test_init();

        let addr = next_addr();
        let _listener = TcpListener::bind(&addr).await.unwrap();
        let good = TcpConnector::from_host_port(addr.ip().to_string(), addr.port());
        assert!(good.healthcheck().await.is_ok());

        let addr = next_addr();
        let bad = TcpConnector::from_host_port(addr.ip().to_string(), addr.port());
        assert!(bad.healthcheck().await.is_err());
    }
}
