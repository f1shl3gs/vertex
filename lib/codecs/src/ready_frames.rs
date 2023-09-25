use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;
use futures_util::StreamExt;

const DEFAULT_CAPACITY: usize = 1024;

/// A stream combinator aimed at improving the performance of decoder streams under load
///
/// This is similar in spirit ot `StreamExt::ready_chunks`, but built specifically for the
/// particular result tuple returned by decoding streams. The more general `FoldReady` is
/// left as an exercise to the reader.
pub struct ReadyFrames<T, U, E> {
    inner: T,
    enqueued: Vec<U>,
    enqueued_size: usize,
    error_slot: Option<E>,
    enqueued_limit: usize,
}

impl<T, U, E> ReadyFrames<T, U, E>
where
    T: Stream<Item = Result<(U, usize), E>> + Unpin,
    U: Unpin,
    E: Unpin,
{
    /// Creates a new `ReadyChunks` by wrapping a decoder stream, most commonly a `FramedRead`
    pub fn new(inner: T) -> Self {
        Self::with_capacity(inner, DEFAULT_CAPACITY)
    }

    /// Creates a new `ReadyChunks` with a specified capacity by wrapping a decoder stream,
    /// most commonly a `FramedRead`
    ///
    /// The specified capacity is a soft limit, and chunks may be returned that contain more
    /// than that number of items
    pub fn with_capacity(inner: T, cap: usize) -> Self {
        Self {
            inner,
            enqueued: Vec::with_capacity(cap),
            enqueued_size: 0,
            error_slot: None,
            enqueued_limit: cap,
        }
    }

    /// Returns a reference to the underlying stream
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    /// Returns a mutable reference to the underlying stream
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    fn flush(&mut self) -> (Vec<U>, usize) {
        let frames = std::mem::take(&mut self.enqueued);
        let size = self.enqueued_size;
        self.enqueued_size = 0;
        (frames, size)
    }
}

impl<T, U, E> Stream for ReadyFrames<T, U, E>
where
    T: Stream<Item = Result<(U, usize), E>> + Unpin,
    U: Unpin,
    E: Unpin,
{
    type Item = Result<(Vec<U>, usize), E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(err) = self.error_slot.take() {
            return Poll::Ready(Some(Err(err)));
        }

        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok((frame, size)))) => {
                    self.enqueued.push(frame);
                    self.enqueued_size += size;
                    if self.enqueued.len() >= self.enqueued_limit {
                        return Poll::Ready(Some(Ok(self.flush())));
                    }
                }

                Poll::Ready(Some(Err(err))) => {
                    return if self.enqueued.is_empty() {
                        Poll::Ready(Some(Err(err)))
                    } else {
                        self.error_slot = Some(err);
                        Poll::Ready(Some(Ok(self.flush())))
                    }
                }

                Poll::Ready(None) => {
                    return if !self.enqueued.is_empty() {
                        Poll::Ready(Some(Ok(self.flush())))
                    } else {
                        Poll::Ready(None)
                    }
                }

                Poll::Pending => {
                    return if !self.enqueued.is_empty() {
                        Poll::Ready(Some(Ok(self.flush())))
                    } else {
                        Poll::Pending
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::{channel::mpsc, SinkExt};
    use futures_util::poll;

    use super::*;

    #[tokio::test]
    async fn idle_passthrough() {
        let (mut tx, rx) = mpsc::channel::<Result<(&str, usize), &str>>(5);
        let mut rf = ReadyFrames::with_capacity(rx, 2);

        assert_eq!(Poll::Pending, poll!(rf.next()));

        tx.send(Ok(("foo", 1))).await.unwrap();

        assert_eq!(Poll::Ready(Some(Ok((vec!["foo"], 1)))), poll!(rf.next()));
        assert_eq!(Poll::Pending, poll!(rf.next()));
    }

    #[tokio::test]
    async fn limits_to_capacity() {
        let (mut tx, rx) = mpsc::channel::<Result<(&str, usize), &str>>(5);
        let mut rf = ReadyFrames::with_capacity(rx, 2);

        tx.send(Ok(("foo", 2))).await.unwrap();
        tx.send(Ok(("bar", 3))).await.unwrap();

        assert_eq!(
            Poll::Ready(Some(Ok((vec!["foo", "bar"], 5)))),
            poll!(rf.next())
        );
        assert_eq!(Poll::Pending, poll!(rf.next()));

        tx.send(Ok(("foo", 4))).await.unwrap();
        tx.send(Ok(("bar", 5))).await.unwrap();
        tx.send(Ok(("baz", 6))).await.unwrap();

        assert_eq!(
            Poll::Ready(Some(Ok((vec!["foo", "bar"], 9)))),
            poll!(rf.next())
        );
        assert_eq!(Poll::Ready(Some(Ok((vec!["baz"], 6)))), poll!(rf.next()));
        assert_eq!(Poll::Pending, poll!(rf.next()));
    }

    #[tokio::test]
    async fn error_passing() {
        let (mut tx, rx) = mpsc::channel::<Result<(&str, usize), &str>>(5);
        let mut rf = ReadyFrames::with_capacity(rx, 2);

        tx.send(Err("oops")).await.unwrap();

        assert_eq!(Poll::Ready(Some(Err("oops"))), poll!(rf.next()));
        assert_eq!(Poll::Pending, poll!(rf.next()));

        tx.send(Ok(("foo", 7))).await.unwrap();
        tx.send(Err("oops")).await.unwrap();

        assert_eq!(Poll::Ready(Some(Ok((vec!["foo"], 7)))), poll!(rf.next()));
        assert_eq!(Poll::Ready(Some(Err("oops"))), poll!(rf.next()));
        assert_eq!(Poll::Pending, poll!(rf.next()));
    }

    #[tokio::test]
    async fn closing() {
        let (mut tx, rx) = mpsc::channel::<Result<(&str, usize), &str>>(5);
        let mut rf = ReadyFrames::with_capacity(rx, 2);

        tx.send(Ok(("foo", 8))).await.unwrap();
        tx.send(Ok(("bar", 9))).await.unwrap();
        tx.send(Ok(("baz", 10))).await.unwrap();
        drop(tx);

        assert_eq!(
            Poll::Ready(Some(Ok((vec!["foo", "bar"], 17)))),
            poll!(rf.next())
        );
        assert_eq!(Poll::Ready(Some(Ok((vec!["baz"], 10)))), poll!(rf.next()));
        assert_eq!(Poll::Ready(None), poll!(rf.next()));
    }
}
