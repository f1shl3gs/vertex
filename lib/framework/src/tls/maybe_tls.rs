use std::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};

pin_project! {
    /// A type wrapper for objects that can exist in either a raw state or
    /// wrapped by TLS handling.
    #[project = MaybeTlsProj]
    pub enum MaybeTls<R, T> {
        Raw{#[pin] raw: R},
        Tls{#[pin] tls: T},
    }
}

impl<T> From<Option<T>> for MaybeTls<(), T> {
    fn from(tls: Option<T>) -> Self {
        match tls {
            Some(tls) => Self::Tls { tls },
            None => Self::Raw { raw: () },
        }
    }
}

// Conditionally implement Clone for Cloneable types
impl<R: Clone, T: Clone> Clone for MaybeTls<R, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Raw { raw } => Self::Raw { raw: raw.clone() },
            Self::Tls { tls } => Self::Tls { tls: tls.clone() },
        }
    }
}

// Conditionally implement Debug for Debuggable types
impl<R: fmt::Debug, T: fmt::Debug> fmt::Debug for MaybeTls<R, T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw { raw } => write!(fmt, "MaybeTls::Raw({:?})", raw),
            Self::Tls { tls } => write!(fmt, "MaybeTls::Tls({:?})", tls),
        }
    }
}

impl<R: AsyncRead, T: AsyncRead> AsyncRead for MaybeTls<R, T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Tls { tls } => tls.poll_read(cx, buf),
            MaybeTlsProj::Raw { raw } => raw.poll_read(cx, buf),
        }
    }
}

impl<R: AsyncWrite, T: AsyncWrite> AsyncWrite for MaybeTls<R, T> {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        match self.project() {
            MaybeTlsProj::Tls { tls } => tls.poll_write(cx, buf),
            MaybeTlsProj::Raw { raw } => raw.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Tls { tls } => tls.poll_flush(cx),
            MaybeTlsProj::Raw { raw } => raw.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        match self.project() {
            MaybeTlsProj::Tls { tls } => tls.poll_shutdown(cx),
            MaybeTlsProj::Raw { raw } => raw.poll_shutdown(cx),
        }
    }
}
