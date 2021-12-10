use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures::ready;
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Wrapper for `tokio::sync::Semaphore` that allows for shrinking the semaphore safely
#[derive(Debug)]
pub struct ShrinkableSemaphore {
    semaphore: Arc<Semaphore>,
    to_forget: Mutex<usize>,
}

impl ShrinkableSemaphore {
    pub fn new(size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            to_forget: Mutex::new(0),
        }
    }

    pub fn acquire(self: Arc<Self>) -> impl Future<Output = OwnedSemaphorePermit> + Send + 'static {
        MaybeForgetFuture {
            master: Arc::clone(&self),
            future: Box::pin(
                Arc::clone(&self.semaphore)
                    .acquire_owned()
                    .map(|r| r.expect("Semaphore has been closed")),
            ),
        }
    }

    pub fn forget_permits(&self, count: usize) {
        // When forgetting permits, there may not be enough immediately available.
        // If so, just increase the count we need to forget later and finish.
        let mut to_forget = self
            .to_forget
            .lock()
            .expect("ShrinkableSemaphore mutex is poisoned");

        for _ in 0..count {
            match self.semaphore.try_acquire() {
                Ok(permtit) => permtit.forget(),
                Err(_) => *to_forget += 1,
            }
        }
    }

    pub fn add_permits(&self, count: usize) {
        let mut to_forget = self
            .to_forget
            .lock()
            .expect("ShrinkableSemaphore mutex is poisoned");

        if *to_forget >= count {
            *to_forget -= count;
        } else {
            self.semaphore.add_permits(count);
            *to_forget = to_forget.saturating_sub(count);
        }
    }
}

/// A future that accounts for the possibility of needing to forget some number
/// of permits before yielding a valid one.
struct MaybeForgetFuture {
    master: Arc<ShrinkableSemaphore>,
    future: BoxFuture<'static, OwnedSemaphorePermit>,
}

impl Future for MaybeForgetFuture {
    type Output = OwnedSemaphorePermit;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let master = Arc::clone(&self.master);
        let mut to_forget = master
            .to_forget
            .lock()
            .expect("Shrinkable semaphore mutex is poisoned");

        while *to_forget > 0 {
            let permit = ready!(self.future.as_mut().poll(cx));
            permit.forget();
            *to_forget -= 1;
            let fut = Arc::clone(&self.master.semaphore)
                .acquire_owned()
                .map(|r| r.expect("Semaphore is closed"));

            drop(std::mem::replace(&mut self.future, Box::pin(fut)));
        }

        drop(to_forget);
        self.future.as_mut().poll(cx)
    }
}
