use std::fmt::Debug;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::OptionFuture;
use futures::stream::{BoxStream, FuturesOrdered, FuturesUnordered};
use futures::{FutureExt, Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::error;

use crate::{BatchStatus, BatchStatusReceiver};

/// The `OrderedFinalizer` framework produces a stream of acknowledged
/// event batch identifiers from a source in a single background task
/// *in the order they are received from the source*, using
/// `FinalizerSet`.
pub type OrderedFinalizer<T> = FinalizerSet<T, FuturesOrdered<FinalizerFuture<T>>>;

/// The `UnorderedFinalizer` framework produces a stream of
/// acknowledged event batch identifiers from a source in a single
/// background task *in the order that finalization happens on the
/// event batches*, using `FinalizerSet`.
pub type UnorderedFinalizer<T> = FinalizerSet<T, FuturesUnordered<FinalizerFuture<T>>>;

/// The `FinalizerSet` framework here is a mechanism for creating a
/// stream of acknowledged (finalized) event batch identifiers from a
/// source as done in a single background task. It does this by
/// pushing the batch status receiver along with an identifier into
/// either a `FuturesOrdered` or `FuturesUnordered`, waiting on the
/// stream of acknowledgements that comes out, extracting just the
/// identifier and sending that into the returned stream. The type `T`
/// is the source-specific data associated with each entry.
#[derive(Debug)]
pub struct FinalizerSet<T, S> {
    sender: UnboundedSender<(BatchStatusReceiver, T)>,
    _phantom: PhantomData<S>,
}

impl<T, S> FinalizerSet<T, S>
where
    T: Send + Debug + 'static,
    S: FuturesSet<FinalizerFuture<T>> + Default + Send + Unpin + 'static,
{
    /// Produce a finalizer set along with the output stream of
    /// received acknowledged batch identifiers.
    #[must_use]
    pub fn new<SS>(shutdown: Option<SS>) -> (Self, BoxStream<'static, (BatchStatus, T)>)
    where
        SS: Future + Send + Unpin + 'static,
        <SS as Future>::Output: Send,
    {
        let (tx, rx) = unbounded_channel();

        (
            Self {
                sender: tx,
                _phantom: PhantomData,
            },
            finalizer_stream(shutdown, rx, S::default()).boxed(),
        )
    }

    /// This returns an optional finalizer set along with a generic
    /// stream of acknowledged identifiers. In the case the finalizer
    /// is not to be used, a special empty stream is returned that is
    /// always pending and so never wakes.
    #[must_use]
    pub fn maybe_new<SS>(
        maybe: bool,
        shutdown: Option<SS>,
    ) -> (Option<Self>, BoxStream<'static, (BatchStatus, T)>)
    where
        SS: Future + Send + Unpin + 'static,
        <SS as Future>::Output: Send,
    {
        if maybe {
            let (finalizer, stream) = Self::new(shutdown);
            (Some(finalizer), stream)
        } else {
            (None, EmptyStream::default().boxed())
        }
    }

    pub fn add(&self, entry: T, receiver: BatchStatusReceiver) {
        if let Err(err) = self.sender.send((receiver, entry)) {
            error!(message = "FinalizerSet task ended prematurely", %err);
        }
    }
}

fn finalizer_stream<S, T, R>(
    shutdown: Option<S>,
    mut new_entries: UnboundedReceiver<(BatchStatusReceiver, T)>,
    mut status_receivers: R,
) -> impl Stream<Item = (BatchStatus, T)>
where
    R: Default + FuturesSet<FinalizerFuture<T>> + Unpin,
    S: Future + Send + Unpin + 'static,
{
    let handle_shutdown = shutdown.is_some();
    let mut shutdown = OptionFuture::from(shutdown);

    async_stream::stream! {
        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown, if handle_shutdown => break,
                // Only poll for new entries until shutdown is flagged.
                new_entry = new_entries.recv() => match new_entry {
                    Some((receiver, entry)) => {
                        status_receivers.push(FinalizerFuture {
                            receiver,
                            entry: Some(entry),
                        });
                    }
                    // The new entry sender went away before shutdown, count it as a shutdown too.
                    None => break,
                },
                finished = status_receivers.next(), if !status_receivers.is_empty() => match finished {
                    Some((status, entry)) => yield (status, entry),
                    // The `is_empty` guard above prevents this from being reachable.
                    None => unreachable!(),
                },
            }
        }

        // We've either seen a shutdown signal or the new entry sender
        // was closed. Wait for the last statuses to come in before
        // indicating we are done.
        while let Some((status, entry)) = status_receivers.next().await {
            yield (status, entry);
        }

        // Hold on to the shutdown signal until here to prevent
        // notification of completion before this stream is done.
        drop(shutdown);
    }
}

pub trait FuturesSet<Fut: Future>: Stream<Item = Fut::Output> {
    fn is_empty(&self) -> bool;

    fn push(&mut self, future: Fut);
}

impl<Fut: Future> FuturesSet<Fut> for FuturesOrdered<Fut> {
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn push(&mut self, future: Fut) {
        Self::push_back(self, future);
    }
}

impl<Fut: Future> FuturesSet<Fut> for FuturesUnordered<Fut> {
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn push(&mut self, future: Fut) {
        Self::push(self, future);
    }
}

pin_project! {
    pub struct FinalizerFuture<T> {
        receiver: BatchStatusReceiver,
        entry: Option<T>,
    }
}

impl<T> Future for FinalizerFuture<T> {
    type Output = (<BatchStatusReceiver as Future>::Output, T);

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let status = std::task::ready!(self.receiver.poll_unpin(ctx));
        // The use of this above in a `Futures{Ordered|Unordered|`
        // will only take this once before dropping the future.
        Poll::Ready((status, self.entry.take().unwrap_or_else(|| unreachable!())))
    }
}

#[derive(Clone, Copy)]
pub struct EmptyStream<T>(PhantomData<T>);

impl<T> Default for EmptyStream<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T> Stream for EmptyStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}
