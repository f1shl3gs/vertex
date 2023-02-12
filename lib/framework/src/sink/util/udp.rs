use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use async_trait::async_trait;
use backoff::ExponentialBackoff;
use bytes::BytesMut;
use codecs::encoding::Transformer;
use configurable::Configurable;
use event::{Event, EventContainer, EventStatus, Events, Finalizable};
use futures::{future::BoxFuture, ready, stream::BoxStream, FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{net::UdpSocket, sync::oneshot, time::sleep};
use tokio_util::codec::Encoder;

use super::SinkBuildError;
use crate::{dns, udp, Healthcheck, Sink, StreamSink};

#[derive(Debug, Error)]
pub enum UdpError {
    #[error("Failed to create UDP listener socket, error = {0:?}.")]
    Bind(std::io::Error),
    #[error("Send error: {0}")]
    Send(std::io::Error),
    #[error("Connect error: {0}")]
    Connect(std::io::Error),
    #[error("No addresses returned.")]
    NoAddresses,
    #[error("Unable to resolve DNS: {0}")]
    Dns(#[from] dns::DnsError),
    #[error("Failed to get UdpSocket back: {0}")]
    ServiceChannelRecv(#[from] oneshot::error::RecvError),
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UdpSinkConfig {
    /// The address to connect to. The address must include a port.
    #[configurable(required)]
    address: String,

    /// The size of the socket's send buffer.
    ///
    /// If set, the value of the setting is passed via the `SO_SNDBUF` option.
    send_buffer_bytes: Option<usize>,
}

impl UdpSinkConfig {
    pub const fn from_address(address: String) -> Self {
        Self {
            address,
            send_buffer_bytes: None,
        }
    }

    fn build_connector(&self) -> crate::Result<UdpConnector> {
        let uri = self.address.parse::<http::Uri>()?;
        let host = uri.host().ok_or(SinkBuildError::MissingHost)?.to_string();
        let port = uri.port_u16().ok_or(SinkBuildError::MissingPort)?;
        Ok(UdpConnector::new(host, port, self.send_buffer_bytes))
    }

    pub fn build_service(&self) -> crate::Result<(UdpService, Healthcheck)> {
        let connector = self.build_connector()?;
        Ok((
            UdpService::new(connector.clone()),
            async move { connector.healthcheck().await }.boxed(),
        ))
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
        let connector = self.build_connector()?;
        let sink = UdpSink::new(connector.clone(), transformer, encoder);

        Ok((
            Sink::Stream(Box::new(sink)),
            async move { connector.healthcheck().await }.boxed(),
        ))
    }
}

#[derive(Clone)]
struct UdpConnector {
    host: String,
    port: u16,
    send_buffer_bytes: Option<usize>,
}

impl UdpConnector {
    const fn new(host: String, port: u16, send_buffer_bytes: Option<usize>) -> Self {
        Self {
            host,
            port,
            send_buffer_bytes,
        }
    }

    const fn fresh_backoff() -> ExponentialBackoff {
        // TODO: make configurable
        ExponentialBackoff::from_millis(2)
            .factor(250)
            .max_delay(Duration::from_secs(60))
    }

    async fn connect(&self) -> Result<UdpSocket, UdpError> {
        let ip = dns::Resolver
            .lookup_ip(self.host.clone())
            .await?
            .next()
            .ok_or(UdpError::NoAddresses)?;

        let addr = SocketAddr::new(ip, self.port);
        let bind_address = find_bind_address(&addr);

        let socket = UdpSocket::bind(bind_address)
            .await
            .map_err(UdpError::Bind)?;

        if let Some(send_buffer_bytes) = self.send_buffer_bytes {
            if let Err(err) = udp::set_send_buffer_size(&socket, send_buffer_bytes) {
                warn!(message = "Failed configuring send buffer size on UDP socket.", %err);
            }
        }

        socket.connect(addr).await.map_err(UdpError::Connect)?;

        Ok(socket)
    }

    async fn connect_backoff(&self) -> UdpSocket {
        let mut backoff = Self::fresh_backoff();
        loop {
            match self.connect().await {
                Ok(socket) => {
                    debug!(message = "Connected");
                    // TODO: metrics
                    return socket;
                }
                Err(err) => {
                    error!(
                        message = "Unable to connect",
                        %err
                    );
                    // TODO: metrics
                    sleep(backoff.next().unwrap()).await;
                }
            }
        }
    }

    async fn healthcheck(&self) -> crate::Result<()> {
        self.connect().await.map(|_| ()).map_err(Into::into)
    }
}

enum UdpServiceState {
    Disconnected,
    Connecting(BoxFuture<'static, UdpSocket>),
    Connected(UdpSocket),
    Sending(oneshot::Receiver<UdpSocket>),
}

pub struct UdpService {
    connector: UdpConnector,
    state: UdpServiceState,
}

impl UdpService {
    const fn new(connector: UdpConnector) -> Self {
        Self {
            connector,
            state: UdpServiceState::Disconnected,
        }
    }
}

impl tower::Service<BytesMut> for UdpService {
    type Response = ();
    type Error = UdpError;
    type Future = BoxFuture<'static, Result<(), Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            self.state = match &mut self.state {
                UdpServiceState::Disconnected => {
                    let connector = self.connector.clone();
                    UdpServiceState::Connecting(Box::pin(async move {
                        connector.connect_backoff().await
                    }))
                }
                UdpServiceState::Connecting(fut) => {
                    let socket = ready!(fut.poll_unpin(cx));
                    UdpServiceState::Connected(socket)
                }
                UdpServiceState::Connected(_) => break,
                UdpServiceState::Sending(fut) => {
                    let socket = match ready!(fut.poll_unpin(cx)) {
                        Ok(socket) => socket,
                        Err(err) => return Poll::Ready(Err(err.into())),
                    };
                    UdpServiceState::Connected(socket)
                }
            };
        }
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, msg: BytesMut) -> Self::Future {
        let (sender, receiver) = oneshot::channel();

        let mut socket =
            match std::mem::replace(&mut self.state, UdpServiceState::Sending(receiver)) {
                UdpServiceState::Connected(socket) => socket,
                _ => panic!("UdpService::poll_ready should be called first"),
            };

        Box::pin(async move {
            // TODO: Add reconnect support as TCP/Unix?
            let result = udp_send(&mut socket, &msg).await.map_err(UdpError::Send);
            let _ = sender.send(socket);
            result
        })
    }
}

struct UdpSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    connector: UdpConnector,
    transformer: Transformer,
    encoder: E,
}

impl<E> UdpSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    fn new(connector: UdpConnector, transformer: Transformer, encoder: E) -> Self {
        Self {
            connector,
            transformer,
            encoder,
        }
    }
}

#[async_trait]
impl<E> StreamSink for UdpSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let mut input = input.peekable();
        let send_events = metrics::register_counter(
            "socket_sent_events_total",
            "The total number of events emitted by this component.",
        )
        .recorder(&[("mode", "udp")]);
        let processed_bytes = metrics::register_counter(
            "processed_bytes_total",
            "The number of bytes processed by the component.",
        )
        .recorder(&[]);
        let sent_errors = metrics::register_counter(
            "send_errors_total",
            "The total number of errors sending messages.",
        )
        .recorder(&[("mode", "udp")]);

        let mut encoder = self.encoder.clone();
        while Pin::new(&mut input).peek().await.is_some() {
            let mut socket = self.connector.connect_backoff().await;
            while let Some(events) = input.next().await {
                for mut event in events.into_events() {
                    self.transformer.transform(&mut event);

                    let finalizers = event.take_finalizers();
                    let mut buf = BytesMut::new();
                    if encoder.encode(event, &mut buf).is_err() {
                        continue;
                    }

                    match udp_send(&mut socket, &buf).await {
                        Ok(()) => {
                            send_events.inc(1);
                            processed_bytes.inc(buf.len() as u64);
                            finalizers.update_status(EventStatus::Delivered);
                        }
                        Err(err) => {
                            sent_errors.inc(1);
                            debug!(
                                message = "UDP socket error",
                                %err,
                                internal_log_rate_secs = 10
                            );
                            finalizers.update_status(EventStatus::Errored);
                            break;
                        }
                    };
                }
            }
        }

        Ok(())
    }
}

async fn udp_send(socket: &mut UdpSocket, buf: &[u8]) -> tokio::io::Result<()> {
    let sent = socket.send(buf).await?;
    if sent != buf.len() {
        let total = buf.len();

        error!(
            message = "Could not send all data in one UDP packet; dropping some data",
            total,
            sent,
            dropped = total - sent,
            internal_log_rate_secs = 30
        );

        // TODO: metrics
        // counter!("connection_send_errors_total", 1, "mode" => "udp");
    }
    Ok(())
}

fn find_bind_address(remote_addr: &SocketAddr) -> SocketAddr {
    match remote_addr {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}
