use std::fmt::Formatter;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use bytes::Bytes;
use codecs::decoding::StreamDecodingError;
use codecs::ReadyFrames;
use configurable::Configurable;
use event::{BatchNotifier, BatchStatus, Event};
use futures::StreamExt;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use listenfd::ListenFd;
use serde::{de, Deserialize, Deserializer, Serialize};
use smallvec::SmallVec;
use socket2::SockRef;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio_util::codec::{Decoder, FramedRead};

use crate::config::{Resource, SourceContext};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::{MaybeTlsIncomingStream, MaybeTlsListener, MaybeTlsSettings, TlsError};

#[derive(Configurable, Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SocketListenAddr {
    SocketAddr(SocketAddr),
    #[serde(deserialize_with = "parse_systemd_fd")]
    SystemFd(usize),
}

impl std::fmt::Display for SocketListenAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SocketAddr(ref addr) => addr.fmt(f),
            Self::SystemFd(index) => write!(f, "system socket ${}", index),
        }
    }
}

impl From<SocketAddr> for SocketListenAddr {
    fn from(addr: SocketAddr) -> Self {
        Self::SocketAddr(addr)
    }
}

impl From<SocketListenAddr> for Resource {
    fn from(addr: SocketListenAddr) -> Self {
        match addr {
            SocketListenAddr::SocketAddr(addr) => Resource::tcp(addr),
            SocketListenAddr::SystemFd(index) => Self::SystemFd(index),
        }
    }
}

fn parse_systemd_fd<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &'de str = Deserialize::deserialize(deserializer)?;
    match s {
        "systemd" => Ok(0),
        s if s.starts_with("systemd#") => s[8..]
            .parse::<usize>()
            .map_err(de::Error::custom)?
            .checked_sub(1)
            .ok_or_else(|| de::Error::custom("systemd indices start from 1, found 0")),
        _ => Err(de::Error::custom("must start with \"systemd\"")),
    }
}

async fn make_listener(
    addr: SocketListenAddr,
    mut listenfd: ListenFd,
    tls: &MaybeTlsSettings,
) -> Option<MaybeTlsListener> {
    match addr {
        SocketListenAddr::SocketAddr(addr) => match tls.bind(&addr).await {
            Ok(listener) => Some(listener),
            Err(err) => {
                error!(
                    message = "Failed to bind to listener socket",
                    %err
                );
                None
            }
        },
        SocketListenAddr::SystemFd(index) => match listenfd.take_tcp_listener(index) {
            Ok(Some(listener)) => match TcpListener::from_std(listener) {
                Ok(listener) => Some(listener.into()),
                Err(err) => {
                    error!(
                        message = "Failed to bind to listener socket",
                        %err
                    );
                    None
                }
            },
            Ok(None) => {
                error!(message = "Failed to take listen FD, not open or already taken");
                None
            }
            Err(err) => {
                error!(
                    message = "Failed to take listen FD",
                    %err
                );
                None
            }
        },
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum TcpSourceAck {
    Ack,
    Error,
    Reject,
}

pub trait TcpSourceAcker {
    fn build_ack(self, ack: TcpSourceAck) -> Option<Bytes>;
}

pub struct TcpNullAcker;

impl TcpSourceAcker for TcpNullAcker {
    // This function builds an acknowledgement from the source data in
    // the acker and the given acknowledgement code
    fn build_ack(self, _ack: TcpSourceAck) -> Option<Bytes> {
        None
    }
}

pub trait TcpSource: Clone + Send + Sync + 'static
where
    <<Self as TcpSource>::Decoder as Decoder>::Item: Send,
{
    // TODO: replace it once this feature become stable and released
    // Should be default `std::io::Error`
    // Right now this is unstable: https://github.com/rust-lang/rust/issues/29661
    type Error: From<io::Error>
        + StreamDecodingError
        + std::fmt::Debug
        + std::fmt::Display
        + Send
        + Unpin;
    type Item: Into<SmallVec<[Event; 1]>> + Send + Unpin;
    type Decoder: Decoder<Item = (Self::Item, usize), Error = Self::Error> + Send + 'static;
    type Acker: TcpSourceAcker + Send;

    fn decoder(&self) -> Self::Decoder;

    fn handle_events(&self, _events: &mut [Event], _host: Bytes, _size: usize) {}

    fn build_acker(&self, item: &[Self::Item]) -> Self::Acker;

    fn run(
        self,
        addr: SocketListenAddr,
        keepalive: Option<TcpKeepaliveConfig>,
        shutdown_timeout: Duration,
        tls: MaybeTlsSettings,
        receive_buffer_bytes: Option<usize>,
        cx: SourceContext,
        acknowledgements: bool,
        max_connections: Option<u32>,
    ) -> crate::Result<crate::Source> {
        let listenfd = ListenFd::from_env();

        Ok(Box::pin(async move {
            let listener = match make_listener(addr, listenfd, &tls).await {
                None => return Err(()),
                Some(listener) => listener,
            };

            info!(
                message = "Listening",
                addr = %listener.local_addr().map(SocketListenAddr::SocketAddr).unwrap_or(addr)
            );

            let tripwire = cx.shutdown.clone();
            let tripwire = async move {
                let _ = tripwire.await;
                sleep(shutdown_timeout).await
            }
            .shared();

            let shutdown_clone = cx.shutdown.clone();

            listener
                .accept_stream_limited(max_connections)
                .take_until(shutdown_clone)
                .for_each(move |(conn, permit)| {
                    let shutdown_signal = cx.shutdown.clone();
                    let tripwire = tripwire.clone();
                    let source = self.clone();
                    let output = cx.output.clone();

                    async move {
                        let socket = match conn {
                            Ok(socket) => socket,
                            Err(err) => {
                                error!(
                                    message = "Failed to accept socket",
                                    %err
                                );
                                return;
                            }
                        };

                        let peer_addr = socket.peer_addr();
                        let tripwire = tripwire
                            .map(move |_| {
                                info!(
                                    message = "Resetting connection(still open)",
                                    after = ?shutdown_timeout
                                );
                            })
                            .boxed();

                        debug!(message = "Accepted a new connection",
                            peer = %peer_addr
                        );

                        let fut = handle_stream(
                            shutdown_signal,
                            socket,
                            keepalive,
                            receive_buffer_bytes,
                            source,
                            tripwire,
                            peer_addr.ip(),
                            output,
                            acknowledgements,
                        );

                        tokio::spawn(fut.map(move |()| {
                            drop(permit);
                        }));
                    }
                })
                .map(Ok)
                .await
        }))
    }
}

async fn handle_stream<T>(
    mut shutdown_signal: ShutdownSignal,
    mut socket: MaybeTlsIncomingStream<TcpStream>,
    keepalive: Option<TcpKeepaliveConfig>,
    receive_buffer_bytes: Option<usize>,
    source: T,
    mut tripwire: BoxFuture<'static, ()>,
    peer: IpAddr,
    mut output: Pipeline,
    acknowledgements: bool,
) where
    <<T as TcpSource>::Decoder as Decoder>::Item: Send,
    T: TcpSource,
{
    tokio::select! {
        result = socket.handshake() => {
            if let Err(err) = result {
                metrics::register_counter("connection_errors_total", "The total number of connection errors for this instance.")
                    .recorder(&[("mode", "tcp")])
                    .inc(1);

                match err {
                    // Specific error that occurs when the other side is only doing
                    // SYN/SYN-ACK connections for healthcheck.
                    // https://github.com/timberio/vector/issues/7318
                    TlsError::Handshake( ref source )
                        if source.code() == openssl::ssl::ErrorCode::SYSCALL
                            && source.io_error().is_none() =>
                    {
                        debug!(
                            message = "Connection error, probably a healthcheck",
                            %err,
                            internal_log_rate_limit = true
                        );
                    },
                    _ => {
                        warn!(
                            message = "Connection error",
                            %err,
                            internal_log_rate_limit = true
                        );
                    }
                }
                return;
            }
        },
        _ = &mut shutdown_signal => {
            return;
        }
    };

    if let Some(keepalive) = keepalive {
        if let Err(err) = socket.set_keepalive(keepalive) {
            warn!(
                message = "Failed configuring TCP keepalive",
                %err
            );
        }
    }

    if let Some(receive_buffer_bytes) = receive_buffer_bytes {
        if let Err(err) = socket.set_receive_buffer_bytes(receive_buffer_bytes) {
            warn!(
                message = "Failed configuring receive buffer size on TCP socket",
                %err
            );
        }
    }

    let reader = FramedRead::new(socket, source.decoder());
    let mut reader = ReadyFrames::new(reader);
    let host = Bytes::from(peer.to_string());

    loop {
        tokio::select! {
            _ = &mut tripwire => break,
            _ = &mut shutdown_signal => {
                debug!(message = "Start graceful shutdown.");
                // Close our write part of TCP socket to signal the other side
                // that it should stop writing and close the channel.
                let socket = reader.get_ref().get_ref();
                if let Some(stream) = socket.get_ref() {
                    let socket = SockRef::from(stream);
                    if let Err(err) = socket.shutdown(std::net::Shutdown::Write) {
                        warn!(message = "Failed in signalling to the other side to close the TCP channel.", %err);
                    }
                } else {
                    // Connection hasn't yet been established so we are done here.
                    debug!(message = "Closing connection that hasn't yet been fully established.");
                    break;
                }
            },
            res = reader.next() => {
                match res {
                    Some(Ok((frames, byte_size))) => {
                        let acker = source.build_acker(&frames);
                        let (batch, receiver) = BatchNotifier::maybe_new_with_receiver(acknowledgements);
                        let mut events = frames.into_iter().flat_map(Into::into).collect::<Vec<Event>>();
                        if let Some(batch) = batch {
                            for event in &mut events {
                                event.add_batch_notifier(batch.clone());
                            }
                        }
                        source.handle_events(&mut events, host.clone(), byte_size);

                        match output.send_batch(events).await {
                            Ok(_) => {
                                let ack = match receiver {
                                    None => TcpSourceAck::Ack,
                                    Some(receiver) => match receiver.await {
                                        BatchStatus::Delivered => TcpSourceAck::Ack,
                                        BatchStatus::Errored => {
                                            warn!(
                                                message = "Error delivering events to sink",
                                                internal_log_rate_limit = true
                                            );
                                            TcpSourceAck::Error
                                        },
                                        BatchStatus::Failed => {
                                            warn!(
                                                message = "Error to deliver events to sink",
                                                internal_log_rate_limit = true
                                            );
                                            TcpSourceAck::Reject
                                        }
                                    }
                                };

                                if let Some(ack_bytes) = acker.build_ack(ack) {
                                    let stream = reader.get_mut().get_mut();
                                    if let Err(err) = stream.write_all(&ack_bytes).await {
                                        warn!(
                                            message = "Error writing acknowledgement, dropping connection",
                                            %err
                                        );
                                        break;
                                    }
                                }

                                if ack != TcpSourceAck::Ack {
                                    break;
                                }
                            }

                            Err(_) => {
                                warn!(
                                    message = "Failed to send event"
                                );

                                break;
                            }
                        }
                    }

                    Some(Err(err)) => {
                        if !<<T as TcpSource>::Error as StreamDecodingError>::can_continue(&err) {
                            warn!(
                                message = "Failed to read data from TCP source",
                                %err
                            );
                            break
                        }
                    }

                    None => {
                        debug!("Connection closed");
                        break
                    },
                }
            }

            else => break
        }
    }
}
