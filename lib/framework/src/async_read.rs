use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use tokio::io::{AsyncRead, ReadBuf};

pub trait VecAsyncReadExt: AsyncRead {
    /// Read data from this reader until the give future resolves
    fn allow_read_until<F>(self, until: F) -> AllowReadUntil<Self, F>
    where
        Self: Sized,
        F: Future<Output = ()>,
    {
        AllowReadUntil {
            reader: self,
            until,
        }
    }
}

impl<S> VecAsyncReadExt for S where S: AsyncRead {}

pin_project! {
    /// A AsyncRead combinator which reads from a reader until a future resolves
    #[derive(Clone, Debug)]
    pub struct AllowReadUntil<S, F> {
        #[pin]
        reader: S,
        #[pin]
        until: F,
    }
}

impl<S, F> AllowReadUntil<S, F> {
    pub const fn get_ref(&self) -> &S {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut S {
        &mut self.reader
    }
}

impl<S, F> AsyncRead for AllowReadUntil<S, F>
where
    S: AsyncRead,
    F: Future<Output = ()>,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.project();

        match this.until.poll(cx) {
            Poll::Ready(_) => Poll::Ready(Ok(())),
            Poll::Pending => this.reader.poll_read(cx, buf),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShutdownSignal;
    use futures::FutureExt;
    use std::fs::remove_file;
    use testify::temp::temp_file;
    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

    #[tokio::test]
    async fn test_read_line_without_shutdown() {
        let shutdown = ShutdownSignal::noop();
        let temp_path = temp_file();
        let write_file = File::create(temp_path.clone()).await.unwrap();
        let read_file = File::open(temp_path.clone()).await.unwrap();

        // Wrapper AsyncRead
        let read_file = read_file.allow_read_until(shutdown.clone().map(|_| ()));

        let mut reader = BufReader::new(read_file);
        let mut writer = BufWriter::new(write_file);

        writer.write_all(b"First line\n").await.unwrap();
        writer.flush().await.unwrap();

        // Test one of the AsyncBufRead extension functions
        let mut line_one = String::new();
        let _ = reader.read_line(&mut line_one).await.unwrap();

        assert_eq!(line_one, "First line\n");

        writer.write_all(b"Second line\n").await.unwrap();
        writer.flush().await.unwrap();

        let mut line_two = String::new();
        let _ = reader.read_line(&mut line_two).await;

        assert_eq!("Second line\n", line_two);

        remove_file(temp_path).unwrap()
    }

    #[tokio::test]
    async fn test_read_line_with_shutdown() {
        let (trigger_shutdown, shutdown, _) = ShutdownSignal::new_wired();
        let temp_path = temp_file();
        let write_file = File::create(&temp_path).await.unwrap();
        let read_file = File::open(&temp_path).await.unwrap();

        // Wrapper AsyncRead
        let read_file = read_file.allow_read_until(shutdown.clone().map(|_| ()));

        let mut reader = BufReader::new(read_file);
        let mut writer = BufWriter::new(write_file);

        writer.write_all(b"First line\n").await.unwrap();
        writer.flush().await.unwrap();

        // Test one of the AsyncBufRead extension functions
        let mut line_one = String::new();
        let _ = reader.read_line(&mut line_one).await.unwrap();

        assert_eq!(line_one, "First line\n");

        drop(trigger_shutdown);

        writer.write_all(b"Second line\n").await.unwrap();
        writer.flush().await.unwrap();

        let mut line_two = String::new();
        let _ = reader.read_line(&mut line_two).await;

        assert_eq!("", line_two);

        remove_file(temp_path).unwrap();
    }
}
