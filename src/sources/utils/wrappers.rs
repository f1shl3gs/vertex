use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// This wraps the inner socket and emits `BytesReceived` with the
/// actual number of bytes read before hnalding framing
#[pin_project]
pub struct AfterRead<T, F> {
    #[pin]
    inner: T,
    after_read: F,
}

impl<T, F> AfterRead<T, F> {
    pub const fn new(inner: T, after_read: F) -> Self {
        Self { inner, after_read }
    }

    #[cfg(feature = "listenfd")]
    pub const fn get_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: AsyncRead, F> AsyncRead for AfterRead<T, F>
where
    F: Fn(usize),
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let this = self.project();
        let result = this.inner.poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            (this.after_read)(buf.filled().len() - before);
        }

        result
    }
}

pub trait AfterReadExt {
    fn after_read<F>(self, after_read: F) -> AfterRead<Self, F>
    where
        Self: Sized;
}

impl<T: AsyncRead + AsyncWrite> AfterReadExt for T {
    fn after_read<F>(self, after_read: F) -> AfterRead<Self, F>
    where
        Self: Sized,
    {
        AfterRead::new(self, after_read)
    }
}
