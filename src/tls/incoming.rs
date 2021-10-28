use std::io;
use std::io::Error;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::future::BoxFuture;
use futures::FutureExt;
use snafu::ResultExt;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsStream};
use crate::tls::{MaybeTLSStream, TLSError, Handshake};

enum StreamState<S> {
    Accepted(MaybeTLSStream<S>),
    Accepting(BoxFuture<'static, Result<TlsStream<S>, TLSError>>),
    AcceptError(String),
    Closed,
}


pub struct MaybeTLSIncomingStream<S> {
    state: StreamState<S>,
    // BoxFuture doesn't allow access to the inner stream, but users
    // of MaybeTlsIncomingStream want access to the peer address while
    // still handshaking, so we have to cache it here.
    peer_addr: SocketAddr,
}

impl MaybeTLSIncomingStream<TcpStream> {
    pub fn new(
        stream: TcpStream,
        addr: SocketAddr,
        acceptor: Option<TlsAcceptor>,
    ) -> Self {
        let state = match acceptor {
            Some(acceptor) => StreamState::Accepting(
                async move {
                    let s = acceptor.accept(stream).await
                        .context(Handshake)?;

                    Ok(s.into())
                }
                    .boxed(),
            ),
            None => StreamState::Accepted(MaybeTLSStream::Raw(stream))
        };

        Self {
            peer_addr: addr,
            state,
        }
    }

    fn poll_io<T, F>(self: Pin<&mut Self>, cx: &mut Context, poll_fn: F) -> Poll<io::Result<T>>
    where
    F: FnOnce(Pin<&mut MaybeTLSStream<TcpStream>>, &mut Context) -> Poll<io::Result<T>>
    {
        let mut this = self.get_mut();
        loop {
            return match &mut this.state {
                StreamState::Accepted(stream) => poll_fn(Pin::new(stream), cx),
                StreamState::Accepting(fut) => {
                    match futures::ready!(fut.as_mut().poll(cx)) {
                        Ok(stream) => {
                            this.state = StreamState::Accepted(MaybeTLSStream::Tls(stream));
                            continue;
                        }

                        Err(err) => {
                            let err = io::Error::new(io::ErrorKind::Other, err);
                            this.state = StreamState::AcceptError(err.to_string());
                            Poll::Ready(Err(err))
                        }
                    }
                }
                StreamState::AcceptError(err) => {
                    Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, err.to_owned())))
                }
                StreamState::Closed => Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()))
            }
        }
    }
}

impl AsyncRead for MaybeTLSIncomingStream<TcpStream> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        self.poll_io(cx, |s, cx| s.poll_read(cx, buf))
    }
}

impl AsyncWrite for MaybeTLSIncomingStream<TcpStream> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        self.poll_io(cx, |s, cx| s.poll_write(cx, buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        self.poll_io(cx, |s, cx| s.poll_flush(cx))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let mut this = self.get_mut();
        match &mut this.state {
            StreamState::Accepted(stream) => {
                match Pin::new(stream).poll_shutdown(cx) {
                    Poll::Ready(Ok(())) => {
                        this.state = StreamState::Closed;
                        Poll::Ready(Ok(()))
                    }

                    poll_result => poll_result
                }
            }

            StreamState::Accepting(fut) => {
                match futures::ready!(fut.as_mut().poll(cx)) {
                    Ok(stream) => {
                        this.state = StreamState::Accepted(MaybeTLSStream::Tls(stream));
                        Poll::Pending
                    },
                    Err(err) => {
                        let err = io::Error::new(io::ErrorKind::Other, err);
                        this.state = StreamState::AcceptError(err.to_string());
                        Poll::Ready(Err(err))
                    }
                }
            }

            StreamState::AcceptError(err) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, err.to_owned()))),

            StreamState::Closed => Poll::Ready(Ok(())),
        }
    }
}