use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::client::legacy::connect::{Connected, Connection};
use hyper_util::rt::TokioIo;
use tokio_rustls::client::TlsStream;

/// A Proxy Stream wrapper
#[allow(clippy::large_enum_variant)]
pub enum ProxyStream<R> {
    NoProxy(R),
    Regular(R),
    Secured(TokioIo<TlsStream<TokioIo<R>>>),
}

impl<R: Read + Write + Unpin> Read for ProxyStream<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            ProxyStream::NoProxy(s) => Pin::new(s).poll_read(cx, buf),
            ProxyStream::Regular(s) => Pin::new(s).poll_read(cx, buf),
            ProxyStream::Secured(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl<R: Read + Write + Unpin> Write for ProxyStream<R> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            ProxyStream::NoProxy(s) => Pin::new(s).poll_write(cx, buf),
            ProxyStream::Regular(s) => Pin::new(s).poll_write(cx, buf),
            ProxyStream::Secured(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        match self.get_mut() {
            ProxyStream::NoProxy(s) => Pin::new(s).poll_write_vectored(cx, bufs),
            ProxyStream::Regular(s) => Pin::new(s).poll_write_vectored(cx, bufs),
            ProxyStream::Secured(s) => Pin::new(s).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            ProxyStream::NoProxy(s) => s.is_write_vectored(),
            ProxyStream::Regular(s) => s.is_write_vectored(),
            ProxyStream::Secured(s) => s.is_write_vectored(),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            ProxyStream::NoProxy(s) => Pin::new(s).poll_flush(cx),
            ProxyStream::Regular(s) => Pin::new(s).poll_flush(cx),
            ProxyStream::Secured(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            ProxyStream::NoProxy(s) => Pin::new(s).poll_shutdown(cx),
            ProxyStream::Regular(s) => Pin::new(s).poll_shutdown(cx),
            ProxyStream::Secured(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

impl<R: Read + Write + Connection + Unpin> Connection for ProxyStream<R> {
    fn connected(&self) -> Connected {
        match self {
            ProxyStream::NoProxy(s) => s.connected(),
            ProxyStream::Regular(s) => s.connected().proxy(true),
            ProxyStream::Secured(s) => s.inner().get_ref().0.inner().connected().proxy(true),
        }
    }
}
