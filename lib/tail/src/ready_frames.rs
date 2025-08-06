use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};

/// A stream combinator aimed at improving the performance of decoder streams
/// under load.
///
/// This is similar in spirit of `StreamExt::ready_chunks`, but built specific
/// for the particular result tuple returned by decoding streams.
pub struct ReadyFrames<S, T, E> {
    inner: S,

    queued: Vec<T>,
    cached: usize,

    max_bytes: usize,
    max_items: usize,

    error: Option<E>,
}

impl<S, T, E> ReadyFrames<S, T, E> {
    pub fn new(inner: S, max_items: usize, max_bytes: usize) -> Self {
        Self {
            inner,
            queued: vec![],
            error: None,

            max_bytes,
            max_items,
            cached: 0,
        }
    }

    fn flush(&mut self) -> (Vec<T>, usize) {
        let items = std::mem::take(&mut self.queued);
        let size = std::mem::take(&mut self.cached);

        (items, size)
    }
}

impl<S, T, E> Stream for ReadyFrames<S, T, E>
where
    S: Stream<Item = Result<(T, usize), E>> + Unpin,
    T: Unpin,
    E: Unpin,
{
    type Item = Result<(Vec<T>, usize), E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(err) = self.error.take() {
            return Poll::Ready(Some(Err(err)));
        }

        loop {
            match self.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok((item, size)))) => {
                    self.queued.push(item);
                    self.cached += size;

                    if self.queued.len() >= self.max_items || self.cached >= self.max_bytes {
                        return Poll::Ready(Some(Ok(self.flush())));
                    }
                }
                Poll::Ready(Some(Err(err))) => {
                    return if self.queued.is_empty() {
                        Poll::Ready(Some(Err(err)))
                    } else {
                        self.error = Some(err);
                        Poll::Ready(Some(Ok(self.flush())))
                    };
                }
                Poll::Ready(None) => {
                    return if self.queued.is_empty() {
                        Poll::Ready(None)
                    } else {
                        Poll::Ready(Some(Ok(self.flush())))
                    };
                }
                Poll::Pending => {
                    return if self.queued.is_empty() {
                        Poll::Pending
                    } else {
                        Poll::Ready(Some(Ok(self.flush())))
                    };
                }
            }
        }
    }
}
