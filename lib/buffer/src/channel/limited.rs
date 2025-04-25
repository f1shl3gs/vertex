use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use futures::Stream;
use futures::task::AtomicWaker;

use super::semaphore::Semaphore;
use crate::Encodable;
use crate::queue::Queue;

/// limited returns an unbounded, bytes limited MPSC channel
#[must_use]
pub fn limited<T>(limited: usize) -> (LimitedSender<T>, LimitedReceiver<T>) {
    let inner = Arc::new(Inner {
        queue: Queue::default(),
        semaphore: Semaphore::new(limited),
        senders: AtomicUsize::new(1),
        recv: AtomicWaker::new(),
    });

    (
        LimitedSender {
            inner: Arc::clone(&inner),
        },
        LimitedReceiver { inner },
    )
}

/// Error returned by `LimitedSender`
#[derive(Debug)]
pub enum Error<T> {
    Closed(T),

    LimitExceeded(T),
}

struct Inner<T> {
    queue: Queue<T>,
    semaphore: Semaphore,
    senders: AtomicUsize,

    recv: AtomicWaker,
}

pub struct LimitedSender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Clone for LimitedSender<T> {
    fn clone(&self) -> Self {
        self.inner.senders.fetch_add(1, Ordering::SeqCst);

        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Debug for LimitedSender<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LimitedSender")
            .field("semaphore", &self.inner.semaphore)
            .field("senders", &self.inner.senders)
            .finish()
    }
}

impl<T> Drop for LimitedSender<T> {
    fn drop(&mut self) {
        if self.inner.senders.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.inner.semaphore.close();
            self.inner.recv.wake();
        }
    }
}

impl<T: Encodable> LimitedSender<T> {
    /// Sends an item into the channel
    ///
    /// # Errors
    ///
    /// If the receiver has disconnected (does not exist anymore), then `Error::Closed(T)`
    /// be returned with the given `item`
    pub async fn send(&self, item: T) -> Result<(), Error<T>> {
        let bytes = item.byte_size();

        if self.inner.semaphore.acquire(bytes).await.is_err() {
            return Err(Error::Closed(item));
        }

        self.inner.queue.push(item);

        // wake receiver if possible
        self.inner.recv.wake();

        Ok(())
    }

    /// Attempts to send an item into the channel
    ///
    /// # Errors
    ///
    /// If the receiver has disconnected (does not exist anymore), then `Error::Closed(T)` will
    /// be returned with the given `item`.
    ///
    /// If the channel has insufficient capacity for the item, then `Error::LimitExceeded(T)`
    /// will be returned with the given `Item`
    pub async fn try_send(&self, item: T) -> Result<(), Error<T>> {
        let bytes = item.byte_size();

        if let Err(_err) = self.inner.semaphore.try_acquire(bytes) {
            return Err(Error::Closed(item));
        }

        self.inner.queue.push(item);
        // wake receiver if possible
        self.inner.recv.wake();

        Ok(())
    }

    #[inline]
    pub fn available_bytes(&self) -> usize {
        self.inner.semaphore.available_permits()
    }
}

pub struct LimitedReceiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Debug for LimitedReceiver<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LimitedReceiver")
            .field("semaphore", &self.inner.semaphore)
            .field("senders", &self.inner.senders)
            .finish()
    }
}

impl<T> Drop for LimitedReceiver<T> {
    fn drop(&mut self) {
        self.inner.semaphore.close();
        if let Some(waker) = self.inner.recv.take() {
            waker.wake();
        }
    }
}

/*
impl<T: Encodable> LimitedReceiver<T> {
    #[inline]
    pub fn recv(&mut self) -> RecvFuture<'_, T> {
        RecvFuture { receiver: self }
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct RecvFuture<'a, T: Encodable> {
    receiver: &'a LimitedReceiver<T>,
}

impl<T: Encodable> Future for RecvFuture<'_, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let receiver = self.receiver;

        match receiver.inner.queue.pop() {
            Ok(Some(item)) => {
                receiver.inner.semaphore.release(item.byte_size());

                return Poll::Ready(Some(item));
            }
            Ok(None) => {
                // the queue is empty
                if receiver.inner.semaphore.closed() {
                    return Poll::Ready(None);
                }
            }
            Err(_) => {
                // inconsistent
            }
        }

        receiver.inner.recv.register(cx.waker());

        Poll::Pending
    }
}
*/

impl<T: Encodable> Stream for LimitedReceiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.queue.pop() {
            Ok(Some(item)) => {
                self.inner.semaphore.release(item.byte_size());

                return Poll::Ready(Some(item));
            }
            Ok(None) => {
                // the queue is empty
                if self.inner.semaphore.closed() {
                    return Poll::Ready(None);
                }
            }
            Err(_) => {
                // inconsistent
            }
        }

        self.inner.recv.register(cx.waker());

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {

    use futures::StreamExt;
    use tokio_test::task::spawn;
    use tokio_test::{assert_pending, assert_ready, assert_ready_err};

    use super::*;
    use crate::tests::Message;

    #[tokio::test]
    async fn send_and_receive() {
        let (tx, mut rx) = limited::<Message>(10);

        tx.send(1.into()).await.unwrap();

        let msg = rx.next().await.unwrap();
        assert_eq!(msg.size, 1);

        for _ in 0..10 {
            tx.send(1.into()).await.unwrap();
        }

        for _ in 0..10 {
            let received = rx.next().await.unwrap();
            assert_eq!(received.size, 1);
        }
    }

    #[tokio::test]
    async fn block_when_full() {
        let (tx, mut rx) = limited::<Message>(1);

        tx.send(1.into()).await.unwrap();

        // now channel is full
        let mut send = spawn(async { tx.send(1.into()).await.unwrap() });
        let mut recv = spawn(async { rx.next().await });
        assert_pending!(send.poll());

        let received = assert_ready!(recv.poll());
        assert_eq!(received, Some(1.into()));

        // received one, send should be ready
        assert_ready!(send.poll());

        let mut recv = spawn(async { rx.next().await });
        let received = assert_ready!(recv.poll());
        assert_eq!(received, Some(1.into()));
    }

    #[tokio::test]
    async fn notified_when_close() {
        let (tx, mut rx) = limited::<Message>(1);
        tx.send(1.into()).await.unwrap();
        drop(tx);

        let mut recv = spawn(async { rx.next().await });
        let received = assert_ready!(recv.poll());
        assert_eq!(received, Some(1.into()));

        let mut recv = spawn(async { rx.next().await });
        let received = assert_ready!(recv.poll());
        assert_eq!(received, None); // None means the receiver is closed
    }

    #[tokio::test]
    async fn multiple_sender() {
        let (tx, mut rx) = limited::<Message>(10);

        let tx2 = tx.clone();
        let tx3 = tx.clone();
        let tx1 = tx;

        tx1.send(1.into()).await.unwrap();
        tx2.send(2.into()).await.unwrap();
        tx3.send(3.into()).await.unwrap();

        for i in 1..4 {
            let got = rx.next().await.unwrap();
            assert_eq!(got, i.into());
        }

        drop(tx1);
        drop(tx2);
        drop(tx3);
        assert!(rx.next().await.is_none());
    }

    #[tokio::test]
    async fn sender_notified_when_block_on_oversize_acquire() {
        let (tx, rx) = limited::<Message>(10);
        assert_eq!(tx.available_bytes(), 10);

        let mut wait = spawn(async move { tx.send(11.into()).await });
        assert_eq!(rx.inner.semaphore.available_permits(), 10);
        assert_pending!(wait.poll());

        drop(rx);

        assert_ready_err!(wait.poll());
    }
}
