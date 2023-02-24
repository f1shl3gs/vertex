#![allow(clippy::module_name_repetitions)]

use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use event::{BatchStatus, BatchStatusReceiver};
use futures::{FutureExt, Stream};
use futures_util::stream::{BoxStream, FuturesOrdered, FuturesUnordered};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::ShutdownSignal;

/// The `OrderedFinalizer` framework produces a stream of acknowledged
/// event batch identifiers from a source in a single background task
/// *in the order they are received from the source*, using `FinalizerSet`.
pub type OrderedFinalizer<T> = FinalizerSet<T, FuturesOrdered<FinalizerFuture<T>>>;

// /// The `UnorderedFinalizer` framework produces a stream of acknowledged event
// /// batch identifiers from a source in a single background task *in the order
// /// that finalization happens on the event batches*, using `FinalizerSet`.
// pub type UnorderedFinalizer<T> = FinalizerSet<T, FuturesUnordered<FinalizerFuture<T>>>;

/// The `FinalizerSet` framework here is a mechanism for creating a stream
/// of acknowledged (finalized) event batch identifiers from a source as
/// done in a single background task. It does this by pushing the batch
/// status receiver along with an identifier into either a `FuturesOrdered`
/// or `FuturesUnordered`, waiting on the stream of acknowledgements that
/// comes out, extracting just the identifier and sending that into the
/// returned stream. The type `T` is the source-specific data associated
/// with each entry.
pub struct FinalizerSet<T, S> {
    sender: Option<UnboundedSender<(BatchStatusReceiver, T)>>,
    _phantom: PhantomData<S>,
}

impl<T, S> FinalizerSet<T, S>
where
    T: Send + Debug + 'static,
    S: FuturesSet<FinalizerFuture<T>> + Default + Send + Unpin + 'static,
{
    /// Produce a finalizer set along with the output stream of received
    /// acknowledged batch identifiers.
    pub fn new(shutdown: ShutdownSignal) -> (Self, impl Stream<Item = (BatchStatus, T)>) {
        let (tx, rx) = mpsc::unbounded_channel();

        (
            Self {
                sender: Some(tx),
                _phantom: PhantomData::default(),
            },
            FinalizerStream {
                shutdown,
                new_entries: rx,
                status_receivers: S::default(),
                is_shutdown: false,
            },
        )
    }

    /// This returns an optional finalizer set along with a generic stream of acknowledged
    /// identifiers. In the case the finalizer is not to be used, a special empty stream
    /// is returned that is always pending and so never wakes.
    #[must_use]
    pub fn maybe_new(
        maybe: bool,
        shutdown: ShutdownSignal,
    ) -> (Option<Self>, BoxStream<'static, (BatchStatus, T)>) {
        if maybe {
            let (finalizer, stream) = Self::new(shutdown);
            (Some(finalizer), stream.boxed())
        } else {
            (None, EmptyStream::default().boxed())
        }
    }

    pub fn add(&self, entry: T, receiver: BatchStatusReceiver) {
        if let Some(sender) = &self.sender {
            if let Err(err) = sender.send((receiver, entry)) {
                error!(
                    message = "FinalizerSet task ended prematurely",
                    %err
                );
            }
        }
    }
}

#[pin_project::pin_project]
struct FinalizerStream<T, S> {
    shutdown: ShutdownSignal,
    new_entries: UnboundedReceiver<(BatchStatusReceiver, T)>,
    status_receivers: S,
    is_shutdown: bool,
}

impl<T, S> Stream for FinalizerStream<T, S>
where
    S: FuturesSet<FinalizerFuture<T>> + Unpin,
    T: Debug,
{
    type Item = (BatchStatus, T);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if !*this.is_shutdown {
            if this.shutdown.poll_unpin(cx).is_ready() {
                *this.is_shutdown = true;
            }

            // Only poll for new entries until shutdown is flagged.
            // Loop over all the ready new entries at once.
            loop {
                match this.new_entries.poll_recv(cx) {
                    Poll::Pending => break,
                    Poll::Ready(Some((receiver, entry))) => {
                        let entry = Some(entry);
                        this.status_receivers
                            .push(FinalizerFuture { receiver, entry });
                    }
                    // The sender went away before shutdown, count it as a shutdown too.
                    Poll::Ready(None) => {
                        *this.is_shutdown = true;
                        break;
                    }
                }
            }
        }

        match this.status_receivers.poll_next_unpin(cx) {
            Poll::Pending => Poll::Pending,
            // The futures set report `None` ready when there are no
            // entries present, but we want it to report pending
            // instead.
            Poll::Ready(None) => {
                if *this.is_shutdown {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
            Poll::Ready(Some((status, entry))) => Poll::Ready(Some((status, entry))),
        }
    }
}

pub trait FuturesSet<F: Future>: Stream<Item = F::Output> {
    fn is_empty(&self) -> bool;

    fn push(&mut self, fut: F);
}

impl<F: Future> FuturesSet<F> for FuturesOrdered<F> {
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn push(&mut self, fut: F) {
        Self::push_back(self, fut);
    }
}

impl<F: Future> FuturesSet<F> for FuturesUnordered<F> {
    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn push(&mut self, fut: F) {
        Self::push(self, fut);
    }
}

#[pin_project::pin_project]
pub struct FinalizerFuture<T> {
    receiver: BatchStatusReceiver,
    entry: Option<T>,
}

impl<T> Future for FinalizerFuture<T> {
    type Output = (<BatchStatusReceiver as Future>::Output, T);

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let status = futures::ready!(self.receiver.poll_unpin(cx));
        // The use of this above in a `Futures{Ordered|Unordered}` will
        // only take this once before dropping the future.
        Poll::Ready((status, self.entry.take().unwrap_or_else(|| unreachable!())))
    }
}

#[derive(Clone, Copy)]
pub struct EmptyStream<T>(PhantomData<T>);

impl<T> Default for EmptyStream<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

impl<T> Stream for EmptyStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}
