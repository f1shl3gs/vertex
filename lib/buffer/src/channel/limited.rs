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

/// `limited` returns a bounded, bytes limited FIFO MPSC channel
pub fn limited<T>(limit: usize) -> (LimitedSender<T>, LimitedReceiver<T>) {
    let semaphore = Semaphore::new(limit);
    let inner = Arc::new(Inner {
        semaphore,
        queue: Queue::default(),
        recv: AtomicWaker::new(),
    });

    (
        LimitedSender {
            limit,
            senders: Arc::new(AtomicUsize::new(1)),
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
    queue: Queue<(usize, T)>,

    semaphore: Semaphore,

    recv: AtomicWaker,
}

pub struct LimitedSender<T> {
    limit: usize,
    senders: Arc<AtomicUsize>,

    inner: Arc<Inner<T>>,
}

impl<T> Clone for LimitedSender<T> {
    fn clone(&self) -> Self {
        self.senders.fetch_add(1, Ordering::SeqCst);

        Self {
            limit: self.limit,
            senders: Arc::clone(&self.senders),
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> Debug for LimitedSender<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LimitedSender")
            .field("semaphore", &self.inner.semaphore)
            .field("senders", &self.senders.load(Ordering::Acquire))
            .finish()
    }
}

impl<T> Drop for LimitedSender<T> {
    fn drop(&mut self) {
        if self.senders.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.inner.semaphore.close();
            self.inner.recv.wake();
        }
    }
}

impl<T: Encodable> LimitedSender<T> {
    pub async fn send(&self, item: T) -> Result<(), Error<T>> {
        let amount = item.byte_size();
        if amount > self.limit {
            return Err(Error::LimitExceeded(item));
        }

        if self.inner.semaphore.acquire(amount).await.is_err() {
            return Err(Error::Closed(item));
        }

        self.inner.queue.push((amount, item));
        self.inner.recv.wake();

        Ok(())
    }

    pub fn try_send(&self, item: T) -> Result<(), Error<T>> {
        let amount = item.byte_size();
        if amount > self.limit {
            return Err(Error::LimitExceeded(item));
        }

        match self.inner.semaphore.try_acquire(amount) {
            Ok(_) => {
                self.inner.queue.push((amount, item));
                self.inner.recv.wake();

                Ok(())
            }
            Err(_) => Err(Error::LimitExceeded(item)),
        }
    }
}

pub struct LimitedReceiver<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Debug for LimitedReceiver<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LimitedReceiver").finish()
    }
}

impl<T: Encodable> Stream for LimitedReceiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.queue.pop() {
                Ok(Some((amount, item))) => {
                    self.inner.semaphore.release(amount);
                    return Poll::Ready(Some(item));
                }
                Ok(None) => {
                    // There are no messages to pop, in this case, register recv task
                    self.inner.recv.register(cx.waker());
                    break;
                }
                Err(_) => {
                    // inconsistent
                    std::hint::spin_loop();
                }
            }
        }

        // Check the queue again after parking to prevent race condition:
        // a message could be added to the queue after previous `pop`
        // before `register` call
        loop {
            match self.inner.queue.pop() {
                Ok(Some((amount, item))) => {
                    self.inner.semaphore.release(amount);
                    break Poll::Ready(Some(item));
                }
                Ok(None) => {
                    if self.inner.semaphore.closed() {
                        break Poll::Ready(None);
                    }

                    break Poll::Pending;
                }
                Err(_) => {
                    // inconsistent
                    std::hint::spin_loop();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use tokio_test::task::spawn;
    use tokio_test::{assert_pending, assert_ready};

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

        assert_eq!(tx1.inner.semaphore.available_permits(), 10);

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn mpsc() {
        let threads = 10;
        let total = 1000;

        let (tx, mut rx) = limited::<Message>(10);
        for tid in 0..threads {
            let tx = tx.clone();

            tokio::spawn(async move {
                for _ in 0..total {
                    let value = rand::random_range(1..7);

                    tx.send(value.into()).await.unwrap();
                }

                println!("thread {} done", tid);
            });
        }
        drop(tx);

        for _i in 0..threads * total {
            match rx.next().await {
                Some(_msg) => {
                    // println!(
                    //     "got {i}th with value {} remaining {}",
                    //     msg.size,
                    //     rx.inner.semaphore.available_permits()
                    // );
                }
                None => panic!("recv failed"),
            }
        }

        assert!(rx.next().await.is_none());
    }
}
