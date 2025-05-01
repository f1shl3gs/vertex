use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::Uri;
use hyper::rt::ReadBufCursor;
use hyper_util::client::legacy::connect::{Connected, Connection};
use hyper_util::rt::TokioIo;
use pin_project_lite::pin_project;
use tokio::io::AsyncWrite;
use tower::Service;

pin_project! {
    pub struct UnixStream {
        #[pin]
        inner: tokio::net::UnixStream,
    }
}

impl hyper::rt::Read for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: ReadBufCursor<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let mut t = TokioIo::new(self.project().inner);
        Pin::new(&mut t).poll_read(cx, buf)
    }
}

impl hyper::rt::Write for UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        self.project().inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.project().inner.poll_shutdown(cx)
    }
}

impl Connection for UnixStream {
    fn connected(&self) -> Connected {
        Connected::new()
    }
}

#[derive(Clone)]
pub struct UnixConnector(PathBuf);

impl UnixConnector {
    pub fn new(path: PathBuf) -> Self {
        UnixConnector(path)
    }
}

impl Service<Uri> for UnixConnector {
    type Response = UnixStream;
    type Error = std::io::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: Uri) -> Self::Future {
        let path = self.0.clone();

        Box::pin(async move {
            let inner = tokio::net::UnixStream::connect(path).await?;

            Ok(UnixStream { inner })
        })
    }
}
