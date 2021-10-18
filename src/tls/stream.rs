use std::fmt;
use std::fmt::Formatter;
use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};
use hyper::client::connect::{Connected, Connection};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::client::TlsStream;

/// A stream that might be protected with TLS
pub enum MaybeHTTPSStream<T> {
    /// A stream over plain text
    HTTP(T),
    /// A stream protected with TLS
    HTTPS(TlsStream<T>),
}

impl<T: AsyncRead + AsyncWrite + Connection + Unpin> Connection for MaybeHTTPSStream<T> {
    fn connected(&self) -> Connected {
        match self {
            MaybeHTTPSStream::HTTP(s) => s.connected(),
            MaybeHTTPSStream::HTTPS(s) => {
                let (tcp, tls) = s.get_ref();
                if tls.get_alpn_protocol() == Some(b"h2") {
                    tcp.connected().negotiated_h2()
                } else {
                    tcp.connected()
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for MaybeHTTPSStream<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            MaybeHTTPSStream::HTTP(..) => f.pad("HTTP(..)"),
            MaybeHTTPSStream::HTTPS(..) => f.pad("HTTPS(..)")
        }
    }
}

impl<T> From<T> for MaybeHTTPSStream<T> {
    fn from(t: T) -> Self {
        MaybeHTTPSStream::HTTP(t)
    }
}

impl<T> From<TlsStream<T>> for MaybeHTTPSStream<T> {
    fn from(inner: TlsStream<T>) -> Self {
        MaybeHTTPSStream::HTTPS(inner)
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for MaybeHTTPSStream<T> {
    #[inline]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match Pin::get_mut(self) {
            MaybeHTTPSStream::HTTP(s) => Pin::new(s).poll_read(cx, buf),
            MaybeHTTPSStream::HTTPS(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for MaybeHTTPSStream<T> {
    #[inline]
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, Error>> {
        match Pin::get_mut(self) {
            MaybeHTTPSStream::HTTP(s) => Pin::new(s).poll_write(cx, buf),
            MaybeHTTPSStream::HTTPS(s) => Pin::new(s).poll_write(cx, buf)
        }
    }

    #[inline]
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match Pin::get_mut(self) {
            MaybeHTTPSStream::HTTP(s) => Pin::new(s).poll_flush(cx),
            MaybeHTTPSStream::HTTPS(s) => Pin::new(s).poll_flush(cx),
        }
    }

    #[inline]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match Pin::get_mut(self) {
            MaybeHTTPSStream::HTTP(s) => Pin::new(s).poll_shutdown(cx),
            MaybeHTTPSStream::HTTPS(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}