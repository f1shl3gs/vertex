use std::io::{self, Error, ErrorKind};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, ready};

use futures::future::BoxFuture;
use futures::{FutureExt, Stream, stream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::TlsError;
use crate::tcp::TcpKeepaliveConfig;
use crate::tls::{MaybeTls, TlsConfig};

pub struct MaybeTlsListener {
    listener: TcpListener,
    acceptor: Option<TlsAcceptor>,
}

impl MaybeTlsListener {
    pub async fn bind(addr: &SocketAddr, tls: Option<&TlsConfig>) -> Result<Self, TlsError> {
        let listener = TcpListener::bind(addr).await.map_err(TlsError::TcpBind)?;

        let acceptor = match tls {
            Some(tls) => {
                let conf = tls.server_config()?;
                let acceptor = TlsAcceptor::from(Arc::new(conf));

                Self {
                    listener,
                    acceptor: Some(acceptor),
                }
            }
            None => MaybeTlsListener {
                listener,
                acceptor: None,
            },
        };

        Ok(acceptor)
    }

    pub async fn accept(&mut self) -> Result<MaybeTlsIncomingStream<TcpStream>, TlsError> {
        self.listener
            .accept()
            .await
            .map(|(stream, peer_addr)| {
                MaybeTlsIncomingStream::new(stream, peer_addr, self.acceptor.clone())
            })
            .map_err(TlsError::IncomingListener)
    }

    async fn into_accept(mut self) -> (Result<MaybeTlsIncomingStream<TcpStream>, TlsError>, Self) {
        (self.accept().await, self)
    }

    #[allow(unused)]
    pub fn accept_stream(
        self,
    ) -> impl Stream<Item = Result<MaybeTlsIncomingStream<TcpStream>, TlsError>> {
        let mut accept = Box::pin(self.into_accept());
        stream::poll_fn(move |context| match accept.as_mut().poll(context) {
            Poll::Ready((item, this)) => {
                accept.set(this.into_accept());
                Poll::Ready(Some(item))
            }
            Poll::Pending => Poll::Pending,
        })
    }

    #[allow(unused)]
    pub fn accept_stream_limited(
        self,
        max_connections: Option<usize>,
    ) -> impl Stream<
        Item = (
            Result<MaybeTlsIncomingStream<TcpStream>, TlsError>,
            Option<OwnedSemaphorePermit>,
        ),
    > {
        let connection_semaphore = max_connections.map(|max| Arc::new(Semaphore::new(max)));

        let mut semaphore_future = connection_semaphore
            .clone()
            .map(|x| Box::pin(x.acquire_owned()));
        let mut accept = Box::pin(self.into_accept());
        stream::poll_fn(move |context| {
            let permit = match semaphore_future.as_mut() {
                Some(semaphore) => match semaphore.as_mut().poll(context) {
                    Poll::Ready(permit) => {
                        semaphore.set(connection_semaphore.clone().unwrap().acquire_owned());
                        permit.ok()
                    }
                    Poll::Pending => return Poll::Pending,
                },
                None => None,
            };
            match accept.as_mut().poll(context) {
                Poll::Ready((item, this)) => {
                    accept.set(this.into_accept());
                    Poll::Ready(Some((item, permit)))
                }
                Poll::Pending => Poll::Pending,
            }
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.listener.local_addr()
    }
}

impl From<TcpListener> for MaybeTlsListener {
    fn from(listener: TcpListener) -> Self {
        Self {
            listener,
            acceptor: None,
        }
    }
}

pub struct MaybeTlsIncomingStream<S> {
    state: StreamState<S>,
    // BoxFuture doesn't allow access to the inner stream, but users
    // of MaybeTlsIncomingStream want access to the peer address while
    // still handshaking, so we have to cache it here.
    peer_addr: SocketAddr,
}

type MaybeTlsStream<S> = MaybeTls<S, TlsStream<S>>;

// TODO: optimize
#[allow(clippy::large_enum_variant)]
enum StreamState<S> {
    Accepted(MaybeTlsStream<S>),
    Accepting(BoxFuture<'static, Result<TlsStream<S>, TlsError>>),
    AcceptError(String),
    Closed,
}

impl<S> MaybeTlsIncomingStream<S> {
    pub const fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// None if connection still hasn't been established.
    pub fn get_ref(&self) -> Option<&S> {
        match &self.state {
            StreamState::Accepted(stream) => Some(match stream {
                MaybeTls::Raw { raw } => raw,
                MaybeTls::Tls { tls } => {
                    let (s, _c) = tls.get_ref();
                    s
                }
            }),
            StreamState::Accepting(_) => None,
            StreamState::AcceptError(_) => None,
            StreamState::Closed => None,
        }
    }
}

impl MaybeTlsIncomingStream<TcpStream> {
    pub(super) fn new(
        stream: TcpStream,
        peer_addr: SocketAddr,
        acceptor: Option<TlsAcceptor>,
    ) -> Self {
        let state = match acceptor {
            Some(acceptor) => StreamState::Accepting(
                async move { acceptor.accept(stream).await.map_err(TlsError::Handshake) }.boxed(),
            ),
            None => StreamState::Accepted(MaybeTlsStream::Raw { raw: stream }),
        };
        Self { state, peer_addr }
    }

    // Explicit handshake method
    pub async fn handshake(&mut self) -> Result<(), TlsError> {
        if let StreamState::Accepting(fut) = &mut self.state {
            let stream = fut.await?;
            self.state = StreamState::Accepted(MaybeTlsStream::Tls { tls: stream });
        }

        Ok(())
    }

    pub fn set_keepalive(&mut self, keepalive: &TcpKeepaliveConfig) -> io::Result<()> {
        let stream = self.get_ref().ok_or_else(|| {
            Error::new(
                ErrorKind::NotConnected,
                "Can't set keepalive on connection that has not been accepted yet",
            )
        })?;

        if let Some(timeout) = keepalive.timeout {
            let config = socket2::TcpKeepalive::new().with_time(timeout);

            crate::tcp::set_keepalive(stream, &config)?;
        }

        Ok(())
    }

    pub fn set_receive_buffer_bytes(&mut self, bytes: usize) -> io::Result<()> {
        let stream = self.get_ref().ok_or_else(|| {
            Error::new(
                ErrorKind::NotConnected,
                "Can't set receive buffer size on connection that has not been accepted yet",
            )
        })?;

        crate::tcp::set_receive_buffer_size(stream, bytes)
    }

    fn poll_io<T, F>(self: Pin<&mut Self>, cx: &mut Context, poll_fn: F) -> Poll<io::Result<T>>
    where
        F: FnOnce(Pin<&mut MaybeTlsStream<TcpStream>>, &mut Context) -> Poll<io::Result<T>>,
    {
        let this = self.get_mut();
        loop {
            return match &mut this.state {
                StreamState::Accepted(stream) => poll_fn(Pin::new(stream), cx),
                StreamState::Accepting(fut) => match ready!(fut.as_mut().poll(cx)) {
                    Ok(stream) => {
                        this.state = StreamState::Accepted(MaybeTlsStream::Tls { tls: stream });
                        continue;
                    }
                    Err(err) => {
                        let err = Error::other(err);
                        this.state = StreamState::AcceptError(err.to_string());
                        Poll::Ready(Err(err))
                    }
                },
                StreamState::AcceptError(err) => Poll::Ready(Err(Error::other(err.to_owned()))),
                StreamState::Closed => Poll::Ready(Err(ErrorKind::BrokenPipe.into())),
            };
        }
    }
}

impl AsyncRead for MaybeTlsIncomingStream<TcpStream> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_io(cx, |s, cx| s.poll_read(cx, buf))
    }
}

impl AsyncWrite for MaybeTlsIncomingStream<TcpStream> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        self.poll_io(cx, |s, cx| s.poll_write(cx, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.poll_io(cx, |s, cx| s.poll_flush(cx))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        match &mut this.state {
            StreamState::Accepted(stream) => match Pin::new(stream).poll_shutdown(cx) {
                Poll::Ready(Ok(())) => {
                    this.state = StreamState::Closed;
                    Poll::Ready(Ok(()))
                }
                poll_result => poll_result,
            },
            StreamState::Accepting(fut) => match ready!(fut.as_mut().poll(cx)) {
                Ok(stream) => {
                    this.state = StreamState::Accepted(MaybeTlsStream::Tls { tls: stream });
                    Poll::Pending
                }
                Err(err) => {
                    let err = Error::other(err);
                    this.state = StreamState::AcceptError(err.to_string());
                    Poll::Ready(Err(err))
                }
            },
            StreamState::AcceptError(err) => Poll::Ready(Err(Error::other(err.to_owned()))),
            StreamState::Closed => Poll::Ready(Ok(())),
        }
    }
}
