use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use async_trait::async_trait;
use buffers::Acker;
use bytes::{Bytes, BytesMut};
use event::Event;
use futures::{future::BoxFuture, ready, stream::BoxStream, FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tokio::{net::UdpSocket, sync::oneshot, time::sleep};

use super::retries::ExponentialBackoff;
use super::SinkBuildError;
use crate::{config::SinkContext, dns, udp, Healthcheck, Sink, StreamSink};

#[derive(Debug, Snafu)]
pub enum UdpError {
    #[snafu(display("Failed to create UDP listener socket, error = {:?}.", source))]
    BindError { source: std::io::Error },
    #[snafu(display("Send error: {}", source))]
    SendError { source: std::io::Error },
    #[snafu(display("Connect error: {}", source))]
    ConnectError { source: std::io::Error },
    #[snafu(display("No addresses returned."))]
    NoAddresses,
    #[snafu(display("Unable to resolve DNS: {}", source))]
    DnsError { source: crate::dns::DnsError },
    #[snafu(display("Failed to get UdpSocket back: {}", source))]
    ServiceChannelRecvError { source: oneshot::error::RecvError },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UdpSinkConfig {
    address: String,
    send_buffer_bytes: Option<usize>,
}

impl UdpSinkConfig {
    pub const fn from_address(address: String) -> Self {
        Self {
            address,
            send_buffer_bytes: None,
        }
    }

    fn build_connector(&self, _cx: SinkContext) -> crate::Result<UdpConnector> {
        let uri = self.address.parse::<http::Uri>()?;
        let host = uri.host().ok_or(SinkBuildError::MissingHost)?.to_string();
        let port = uri.port_u16().ok_or(SinkBuildError::MissingPort)?;
        Ok(UdpConnector::new(host, port, self.send_buffer_bytes))
    }

    pub fn build_service(&self, cx: SinkContext) -> crate::Result<(UdpService, Healthcheck)> {
        let connector = self.build_connector(cx)?;
        Ok((
            UdpService::new(connector.clone()),
            async move { connector.healthcheck().await }.boxed(),
        ))
    }

    pub fn build(
        &self,
        cx: SinkContext,
        encode_event: impl Fn(Event) -> Option<Bytes> + Send + Sync + 'static,
    ) -> crate::Result<(Sink, Healthcheck)> {
        let connector = self.build_connector(cx.clone())?;
        let sink = UdpSink::new(connector.clone(), cx.acker(), encode_event);
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
            .await
            .context(DnsError)?
            .next()
            .ok_or(UdpError::NoAddresses)?;

        let addr = SocketAddr::new(ip, self.port);
        let bind_address = find_bind_address(&addr);

        let socket = UdpSocket::bind(bind_address).await.context(BindError)?;

        if let Some(send_buffer_bytes) = self.send_buffer_bytes {
            if let Err(err) = udp::set_send_buffer_size(&socket, send_buffer_bytes) {
                warn!(message = "Failed configuring send buffer size on UDP socket.", %err);
            }
        }

        socket.connect(addr).await.context(ConnectError)?;

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
                    let socket = match ready!(fut.poll_unpin(cx)).context(ServiceChannelRecvError) {
                        Ok(socket) => socket,
                        Err(err) => return Poll::Ready(Err(err)),
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
            let result = udp_send(&mut socket, &msg).await.context(SendError);
            let _ = sender.send(socket);
            result
        })
    }
}

struct UdpSink {
    connector: UdpConnector,
    acker: Acker,
    encode_event: Box<dyn Fn(Event) -> Option<Bytes> + Send + Sync>,
}

impl UdpSink {
    fn new(
        connector: UdpConnector,
        acker: Acker,
        encode_event: impl Fn(Event) -> Option<Bytes> + Send + Sync + 'static,
    ) -> Self {
        Self {
            connector,
            acker,
            encode_event: Box::new(encode_event),
        }
    }
}

#[async_trait]
impl StreamSink for UdpSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        let mut input = input.peekable();

        while Pin::new(&mut input).peek().await.is_some() {
            let mut socket = self.connector.connect_backoff().await;
            while let Some(event) = input.next().await {
                self.acker.ack(1);

                let input = match (self.encode_event)(event) {
                    Some(input) => input,
                    None => continue,
                };

                match udp_send(&mut socket, &input).await {
                    Ok(()) => {
                        counter!("socket_event_send_total", 1, "mode" => "udp");
                        counter!("processed_bytes_total", input.len() as u64);
                    }
                    Err(err) => {
                        counter!("socket_event_send_errors_total", 1, "mode" => "udp");

                        debug!(
                            message = "UDP socket error",
                            %err,
                            internal_log_rate_secs = 10
                        );

                        break;
                    }
                };
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

        counter!("connection_send_errors_total", 1, "mode" => "udp");
    }
    Ok(())
}

fn find_bind_address(remote_addr: &SocketAddr) -> SocketAddr {
    match remote_addr {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}
