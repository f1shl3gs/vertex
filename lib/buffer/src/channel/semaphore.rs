use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::queue::Queue;

const MAX_PERMITS: usize = usize::MAX >> 3;
const PERMIT_SHIFT: usize = 1;
const CLOSED: usize = 1;

/// An asynchronous counting semaphore which permits waiting on multiple permits at once.
pub struct Semaphore {
    permits: AtomicUsize,
    queue: Queue<(usize, Waker)>,
}

impl Debug for Semaphore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Semaphore")
            .field("permits", &self.permits)
            .finish_non_exhaustive()
    }
}

impl Semaphore {
    /// Creates a new semaphore with the initial number of permits
    pub fn new(permits: usize) -> Self {
        assert!(
            permits <= MAX_PERMITS,
            "a semaphore may not have more than MAX_PERMITS permits ({MAX_PERMITS})",
        );

        Semaphore {
            permits: AtomicUsize::new(permits << PERMIT_SHIFT),
            queue: Queue::default(),
        }
    }

    pub fn acquire(&self, amount: usize) -> AcquireFuture<'_> {
        AcquireFuture {
            semaphore: self,
            amount,
        }
    }

    pub fn try_acquire(&self, amount: usize) -> Result<(), ()> {
        let needed = amount << PERMIT_SHIFT;

        let mut current = self.permits.load(Ordering::Acquire);
        loop {
            if current & CLOSED == CLOSED {
                return Err(());
            }

            // Are there enough permits remaining?
            if current < needed {
                return Err(());
            }

            let next = current - amount;
            match self
                .permits
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }

    /// Adds `amount` new permits to the semaphore.
    pub fn release(&self, mut amount: usize) {
        self.permits
            .fetch_add(amount << PERMIT_SHIFT, Ordering::Release);

        // wake as many as possible
        //
        //
        while let Ok(Some((acquire, waker))) =
            self.queue.pop_if(|(acquire, _waker)| amount >= *acquire)
        {
            amount -= acquire;
            waker.wake();
        }
    }

    /// Closes the semaphore. This prevents the semaphore from issuing new permits
    /// and notifies all pending waiters.
    pub fn close(&self) {
        self.permits.fetch_or(CLOSED, Ordering::Release);

        while let Ok(Some((_permit, waker))) = self.queue.pop() {
            waker.wake();
        }
    }

    pub fn closed(&self) -> bool {
        let current = self.permits.load(Ordering::Acquire);
        current & CLOSED == CLOSED
    }

    /// Returns the current number of available permits.
    pub fn available_permits(&self) -> usize {
        self.permits.load(Ordering::Acquire) >> PERMIT_SHIFT
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AcquireFuture<'a> {
    semaphore: &'a Semaphore,
    amount: usize,
}

impl Future for AcquireFuture<'_> {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let needed = self.amount << 1;

        // First, try to take the requested number of permits from the semaphore
        let mut current = self.semaphore.permits.load(Ordering::Acquire);

        loop {
            if current & CLOSED > 0 {
                return Poll::Ready(Err(()));
            }

            let next = if current >= needed {
                current - needed
            } else {
                self.semaphore.queue.push((self.amount, cx.waker().clone()));
                return Poll::Pending;
            };

            match self.semaphore.permits.compare_exchange(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Poll::Ready(Ok(()));
                }
                Err(actual) => {
                    current = actual;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio_test::task::spawn;
    use tokio_test::{assert_pending, assert_ready, assert_ready_err};

    use super::*;

    #[tokio::test]
    async fn simple() {
        let sem = Semaphore::new(10);

        let mut acquire = spawn(async { sem.acquire(1).await.unwrap() });
        assert_ready!(acquire.poll());
        let mut acquire = spawn(async { sem.acquire(10).await.unwrap() });
        assert_pending!(acquire.poll());

        sem.release(1);

        assert_ready!(acquire.poll());
    }

    #[tokio::test]
    async fn available_permits() {
        let semaphore = Semaphore::new(10);
        assert_eq!(semaphore.available_permits(), 10);

        semaphore.acquire(1).await.unwrap();
        assert_eq!(semaphore.available_permits(), 9);
        semaphore.acquire(1).await.unwrap();
        assert_eq!(semaphore.available_permits(), 8);

        semaphore.release(1);
        assert_eq!(semaphore.available_permits(), 9);
        semaphore.release(1);
        assert_eq!(semaphore.available_permits(), 10);
    }

    #[tokio::test]
    async fn waits() {
        let semaphore = Semaphore::new(10);
        assert_eq!(semaphore.available_permits(), 10);

        semaphore.acquire(10).await.unwrap();
        assert_eq!(semaphore.available_permits(), 0);

        let tasks = (0..10)
            .map(|_| spawn(async { semaphore.acquire(1).await }))
            .collect::<Vec<_>>();

        for mut task in tasks {
            assert_pending!(task.poll())
        }
    }

    #[tokio::test]
    async fn close() {
        let semaphore = Semaphore::new(5);
        assert_eq!(semaphore.available_permits(), 5);

        semaphore.acquire(1).await.unwrap();
        assert_eq!(semaphore.available_permits(), 4);

        let mut waiting = spawn(async { semaphore.acquire(5).await });
        assert_pending!(waiting.poll());

        semaphore.close();
        assert!(semaphore.acquire(1).await.is_err());
        assert_ready_err!(waiting.poll());
    }
}
