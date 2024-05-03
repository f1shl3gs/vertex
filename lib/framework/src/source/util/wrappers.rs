use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pin_project! {
    /// This wraps the inner socket and emits `BytesReceived` with the
    /// actual number of bytes read before handing framing
    pub struct AfterRead<T, F> {
        #[pin]
        inner: T,
        after_read: F,
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
