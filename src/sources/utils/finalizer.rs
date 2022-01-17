use crate::shutdown::ShutdownSignal;
use event::BatchStatusReceiver;
use futures::{FutureExt, StreamExt};
use futures_util::future::Shared;
use futures_util::stream::FuturesOrdered;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use tokio::sync::mpsc;

pub struct OrderedFinalizer<T> {
    sender: Option<mpsc::UnboundedSender<(BatchStatusReceiver, T)>>,
}

impl<T: Send + 'static> OrderedFinalizer<T> {
    pub fn new(shutdown: Shared<ShutdownSignal>, apply_done: impl Fn(T) + Send + 'static) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(run_finalizer(shutdown, receiver, apply_done));
        Self {
            sender: Some(sender),
        }
    }

    pub fn add(&self, entry: T, receiver: BatchStatusReceiver) {
        if let Some(sender) = &self.sender {
            if let Err(err) = sender.send((receiver, entry)) {
                error!(
                    message = "OrderedFinalizer task ended prematurely",
                    %err
                );
            }
        }
    }
}

async fn run_finalizer<T>(
    shutdown: Shared<ShutdownSignal>,
    mut entries: mpsc::UnboundedReceiver<(BatchStatusReceiver, T)>,
    apply_done: impl Fn(T),
) {
    let mut status_receiver = FuturesOrdered::default();

    loop {
        tokio::select! {
            _ = shutdown.clone() => break,
            entry = entries.recv() => match entry {
                Some((receiver, entry)) => {
                    status_receiver.push(FinalizerFuture {
                        receiver,
                        entry: Some(entry)
                    });
                }

                None => break
            },

            finished = status_receiver.next(), if !status_receiver.is_empty() => match finished {
                Some((_status, entry)) => apply_done(entry),
                // This is_empty guard above prevents this from being reachable.
                None => unreachable!()
            }
        }
    }

    // We've either seen a shutdown signal or the new entry sender was closed.
    // Wait for the last statuses to come in before indicating we are done.
    while let Some((_status, entry)) = status_receiver.next().await {
        apply_done(entry);
    }

    drop(shutdown);
}

#[pin_project::pin_project]
struct FinalizerFuture<T> {
    receiver: BatchStatusReceiver,
    entry: Option<T>,
}

impl<T> Future for FinalizerFuture<T> {
    type Output = (<BatchStatusReceiver as Future>::Output, T);
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let status = futures::ready!(self.receiver.poll_unpin(cx));

        // The use of this above in a `FuturesOrdered` will only take
        // this once before dropping the future.
        Poll::Ready((status, self.entry.take().unwrap_or_else(|| unreachable!())))
    }
}
