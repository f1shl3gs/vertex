use std::fmt::Debug;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use crate::queue::Queue;

const MAX_PERMITS: usize = usize::MAX >> 3;
const PERMIT_SHIFT: usize = 1;
const CLOSED: usize = 1;

pub struct AcquireFuture<'a> {
    amount: usize,
    semaphore: &'a Semaphore,
}

impl<'a> Future for AcquireFuture<'a> {
    type Output = Result<(), ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let needed = self.amount << PERMIT_SHIFT;
        let mut current = self.semaphore.state.load(Ordering::Acquire);
        loop {
            if current & CLOSED == CLOSED {
                return Poll::Ready(Err(()));
            }

            let next = if current >= needed {
                current - needed
            } else {
                // add this task to pending queue, and it will be waked when
                // receiver got a msg
                self.semaphore
                    .pending
                    .push((self.amount, cx.waker().clone()));

                return Poll::Pending;
            };

            match self.semaphore.state.compare_exchange(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Poll::Ready(Ok(())),
                Err(actual) => current = actual,
            }
        }
    }
}

pub struct Semaphore {
    /// a state to track permits and close flag
    /// | 0 ... 62 |   63   |
    /// | permits  | closed |
    state: AtomicUsize,
    pending: Queue<(usize, Waker)>,
}

impl Debug for Semaphore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.load(Ordering::Acquire);

        f.debug_struct("Semaphore")
            .field("permits", &(state >> PERMIT_SHIFT))
            .field("state", &(state & CLOSED == CLOSED))
            .finish()
    }
}

impl Semaphore {
    pub fn new(permits: usize) -> Self {
        debug_assert!(
            permits > 0 && permits <= MAX_PERMITS,
            "limit must be in (0, {MAX_PERMITS}]"
        );

        Self {
            state: AtomicUsize::new(permits << PERMIT_SHIFT),
            pending: Queue::default(),
        }
    }

    #[cfg(test)]
    pub fn available_permits(&self) -> usize {
        self.state.load(Ordering::Acquire) >> PERMIT_SHIFT
    }

    #[inline]
    pub fn close(&self) {
        self.state.fetch_or(CLOSED, Ordering::AcqRel);
    }

    pub fn closed(&self) -> bool {
        self.state.load(Ordering::Acquire) & CLOSED == CLOSED
    }

    pub fn acquire(&self, amount: usize) -> AcquireFuture<'_> {
        AcquireFuture {
            amount,
            semaphore: self,
        }
    }

    pub fn try_acquire(&self, amount: usize) -> Result<(), ()> {
        let needed = amount << PERMIT_SHIFT;

        let mut current = self.state.load(Ordering::Acquire);
        loop {
            if current & CLOSED == CLOSED {
                return Err(());
            }

            if current < needed {
                return Err(());
            }

            let next = current - needed;
            match self
                .state
                .compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }

    pub fn release(&self, amount: usize) {
        let available = self
            .state
            .fetch_add(amount << PERMIT_SHIFT, Ordering::Release)
            + amount;

        loop {
            match self.pending.pop_if(|(acquire, _)| available >= *acquire) {
                Ok(Some((_, waker))) => {
                    waker.wake();
                    break;
                }
                Ok(None) => {
                    break;
                }
                Err(_) => {
                    // inconsistent
                    std::hint::spin_loop();
                }
            }
        }
    }
}
