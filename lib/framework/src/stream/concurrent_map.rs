use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::{Fuse, FuturesOrdered};
use futures::{ready, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::task::JoinHandle;

pin_project! {
    pub struct ConcurrentMap<S, T>
    where
        S: Stream,
        T: Send,
        T: 'static
    {
        #[pin]
        stream: Fuse<S>,
        limit: Option<NonZeroUsize>,
        inflight: FuturesOrdered<JoinHandle<T>>,
        f: Box<dyn Fn(S::Item) -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send>,
    }
}

impl<S, T> ConcurrentMap<S, T>
where
    S: Stream,
    T: Send + 'static,
{
    pub fn new<F>(stream: S, limit: Option<NonZeroUsize>, f: F) -> Self
    where
        F: Fn(S::Item) -> Pin<Box<dyn Future<Output = T> + Send + 'static>> + Send + 'static,
    {
        Self {
            stream: stream.fuse(),
            limit,
            inflight: FuturesOrdered::new(),
            f: Box::new(f),
        }
    }
}

impl<S, T> Stream for ConcurrentMap<S, T>
where
    S: Stream,
    T: Send + 'static,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // The underlying stream is done, and we have no more inflight futures.
        if this.stream.is_done() && this.inflight.is_empty() {
            return Poll::Ready(None);
        }

        loop {
            let can_poll_stream = match this.limit {
                None => true,
                Some(limit) => this.inflight.len() < limit.get(),
            };

            if can_poll_stream {
                match this.stream.as_mut().poll_next(cx) {
                    // Even if there's no items from the underlying stream, we still have the
                    // inflight futures to check, so we don't return just yet.
                    Poll::Pending | Poll::Ready(None) => break,
                    Poll::Ready(Some(item)) => {
                        let fut = (this.f)(item);
                        let handle = tokio::spawn(fut);
                        this.inflight.push_back(handle);
                    }
                }
            } else {
                // We're at out inflight limit, so stop generating tasks for the moment.
                break;
            }
        }

        match ready!(this.inflight.poll_next_unpin(cx)) {
            // Either nothing is inflight, or nothing is ready
            None => Poll::Pending,
            Some(result) => match result {
                Ok(item) => Poll::Ready(Some(item)),
                Err(err) => {
                    if let Ok(reason) = err.try_into_panic() {
                        // Resume the panic here on the calling task
                        std::panic::resume_unwind(reason);
                    } else {
                        // The task was cancelled, which makes no sense, because we hold the join
                        // handle. Only sensible thing to do is panic, because this is a bug.
                        panic!("concurrent map task cancelled outside of our control");
                    }
                }
            },
        }
    }
}
