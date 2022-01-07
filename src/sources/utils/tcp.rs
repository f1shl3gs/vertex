use crate::config::SourceContext;
use crate::shutdown::ShutdownSignal;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::{MaybeTlsIncomingStream, MaybeTlsListener, MaybeTlsSettings};
use bytes::Bytes;
use event::{Event, BatchStatus, BatchNotifier};
use futures::Sink;
use futures_util::future::BoxFuture;
use futures_util::{FutureExt, SinkExt, StreamExt};
use listenfd::ListenFd;
use serde::{de, Deserialize, Deserializer};
use std::fmt::{Formatter, Pointer};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tokio_util::codec::{Decoder, FramedRead};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
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
                )
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
    fn build_ack(self, ack: TcpSourceAck) -> Option<Bytes> {
        None
    }
}

pub trait TcpSource: Clone + Send + Sync + 'static
where
    <<Self as TcpSource>::Decoder as tokio_util::codec::Decoder>::Item: std::marker::Send,
{
    // TODO: replace it once this feature become stable and released
    // Should be default `std::io::Error`
    // Right now this is unstable: https://github.com/rust-lang/rust/issues/29661
    type Error: From<io::Error> + StreamDecodingError + std::fmt::Debug + std::fmt::Display + Send;
    type Item: Into<SmallVec<[Event; 1]>> + Send;
    type Decoder: Decoder<Item = (Self::Item, usize), Error = Self::Error> + Send + 'static;
    type Acker: TcpSourceAcker + Send;

    fn decoder(&self) -> Self::Decoder;

    fn handle_events(&self, events: &mut [Event], host: Bytes, size: usize) {}

    fn build_acker(&self, item: &Self::Item) -> Self::Acker;

    fn run(
        self,
        addr: SocketListenAddr,
        keepalive: Option<TcpKeepaliveConfig>,
        shutdown_timeout: Duration,
        tls: MaybeTlsSettings,
        receive_buffer_bytes: Option<usize>,
        ctx: SourceContext,
        acknowledgements: bool,
    ) -> crate::Result<crate::sources::Source> {
        let listenfd = ListenFd::from_env();
        let output = ctx
            .out
            .sink_map_err(|err| error!(message = "Error sending event", %err));

        Ok(Box::pin(async move {
            let listener = match make_listener(addr, listenfd, &tls).await {
                None => return Err(()),
                Some(listener) => listener,
            };

            info!(
                message = "Listening",
                addr = %listener.local_addr().map(SocketListenAddr::SocketAddr).unwrap_or(addr)
            );

            let tripwire = ctx.shutdown.clone();
            let tripwire = async move {
                let _ = tripwire.await;
                sleep(shutdown_timeout).await
            }
            .shared();

            let shutdown_clone = ctx.shutdown.clone();

            listener
                .accept_stream()
                .take_until(shutdown_clone)
                .for_each(move |conn| {
                    let shutdown_signal = ctx.shutdown.clone();
                    let tripwire = tripwire.clone();
                    let source = self.clone();
                    let output = output.clone();

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
                            acknowledgements
                        );

                        tokio::spawn(
                            fut
                        );
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
    mut out: impl Sink<Event> + Send + 'static + Unpin,
    acknowledgements: bool,
) where
    <<T as TcpSource>::Decoder as tokio_util::codec::Decoder>::Item: std::marker::Send,
    T: TcpSource,
{
    tokio::select! {
        result = socket.handshake() => {
            if let (err) = result {
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

    let mut reader = FramedRead::new(socket, source.decoder());
    let host = Bytes::from(peer.to_string());

    loop {
        tokio::select! {
            _ = &mut tripwire => break,
            _ = &mut shutdown_signal => {
                debug!(message = "Start graceful shutdown");
                if let Err(err) = socket.shutdown(std::net::Shutdown::Write) {
                    warn!(
                        message = "Failed in signalling to the other side to close the TCP channel",
                        %err
                    );
                }
            },
            res = reader.next() => {
                match res {
                    Some(Ok((item, byte_size))) => {
                        let acker = source.build_acker(&item);
                        let (batch, receiver) = BatchNotifier::maybe_new_with_receiver(acknowledgements);
                        let mut events = item.into();
                        if let Some(batch) = batch {
                            for event in &mut events {
                                event.add_batch_notifier(Arc::clone(&batch));
                            }
                        }

                        source.handle_events(&mut events, host.clone(), byte_size);
                        match out.send_all(&mut stream::iter(events).map(Ok)).awiat {
                            Ok(_) => {
                                let ack = match receiver {
                                    None => TcpSourceAck::Ack,
                                    Some(receiver) => match receiver.await {
                                        BatchStatus::Delivered => TcpSourceAck::Ack,
                                        BatchStatus::Errored => {
                                            warn!(
                                                message = "Error delivering events to sink",
                                                internal_log_rate_secs = 5
                                            );
                                            TcpSourceAck::Error
                                        },
                                        BatchStatus::Failed => {
                                            warn!(
                                                message = "Error to deliver events to sink",
                                                internal_log_rate_secs = 5
                                            );
                                            TcpSourceAck::Reject
                                        }
                                    }
                                };

                                if let Some(ack_bytes) = acker.build_ack(ack) {
                                    let stream = reader.get_mut();
                                    if let Err(err) = stream.write_all(&ack_bytes).await {
                                        warn!(
                                            message = "Error writing acknowledgement, dropping connection",
                                            %err
                                        );
                                        break;
                                    }
                                }

                                if ack != TcpSourceAck {
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
