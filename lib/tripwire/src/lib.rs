#![deny(clippy::pedantic)]

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering::SeqCst};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

const CANCELLATION_MASK: u64 = 1;
const NEXT_RECEIVER_MASK: u64 = !CANCELLATION_MASK;

const CANCELLED: u64 = 1;

fn get_cancellation(data: u64) -> bool {
    CANCELLED == data & CANCELLATION_MASK
}

fn set_state(data: u64, cancelled: u64) -> u64 {
    (data & NEXT_RECEIVER_MASK) | (cancelled & CANCELLATION_MASK)
}

fn get_next_id(data: u64) -> u64 {
    (data & NEXT_RECEIVER_MASK) >> 1
}

fn inc_next_id(data: u64) -> u64 {
    data + (1 << 1)
}

struct Shared {
    // `state` uses 1 bits to store cancellation state,
    // the reset of the bits are used to store the next
    // id of receiver.
    state: AtomicU64,

    // each receiver hold an id, and when the receiver
    // drops the waker will be removed from this map.
    wakers: Mutex<HashMap<u64, Waker>>,
}

impl Shared {
    #[inline]
    fn get_cancelled(&self) -> bool {
        get_cancellation(self.state.load(SeqCst))
    }

    fn set_cancelled(&self) {
        let mut curr = self.state.load(SeqCst);

        loop {
            let next = set_state(curr, CANCELLED);
            match self.state.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => return,
                Err(actual) => curr = actual,
            }
        }
    }

    fn next_id(&self) -> u64 {
        let mut curr = self.state.load(SeqCst);

        loop {
            let next = inc_next_id(curr);

            match self.state.compare_exchange(curr, next, SeqCst, SeqCst) {
                Ok(_) => return get_next_id(next),
                Err(actual) => curr = actual,
            }
        }
    }

    fn wake_all(&self) {
        self.wakers
            .lock()
            .expect("lock waker map success")
            .drain()
            .for_each(|(_key, waker)| waker.wake());
    }
}

/// A handle to a set of cancellable tripwire.
///
/// If the `Trigger` is dropped, any tripwire associated with it are resolved (this is equivalent
/// to calling [`Trigger::cancel`]. To override this behavior, call [`Trigger::disable`].
pub struct Trigger {
    shared: Option<Arc<Shared>>,
}

impl Trigger {
    /// Cancel all associated tripwire, make them immediately resolved.
    pub fn cancel(self) {
        drop(self);
    }

    /// Disable the `Trigger`, and leave all associated `Tripwire` pending forever.
    pub fn disable(mut self) {
        self.shared.take();
        drop(self);
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        if let Some(shared) = self.shared.take() {
            shared.set_cancelled();
            shared.wake_all();
        }
    }
}

/// A `Tripwire` is a convenient mechanism for implementing graceful shutdown over many
/// asynchronous streams. A `Tripwire` is a `Future` that is `Clone`, and that can be passed to
/// [`StreamExt::take_until`]. All `Tripwire` clones are associated with the same [`Trigger`],
/// which is then used to signal that all the associated streams should be terminated.
///
/// The `Tripwire` future resolves if the stream should be considered closed.
pub struct Tripwire {
    /// id used to indicate the waker store in `Shared`.
    id: u64,

    shared: Arc<Shared>,
}

impl Clone for Tripwire {
    fn clone(&self) -> Self {
        let shared = self.shared.clone();
        let id = shared.next_id();

        Self { id, shared }
    }
}

impl Drop for Tripwire {
    fn drop(&mut self) {
        // If the `Tripwire` is never polled, this remove might fail, but
        // it is ok.
        self.shared
            .wakers
            .lock()
            .expect("lock waker map success")
            .remove(&self.id);
    }
}

impl Future for Tripwire {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let cancelled = self.shared.get_cancelled();
        if cancelled {
            return Poll::Ready(());
        }

        // Register current waker so that the `Trigger` can wake up this task when
        // `Tripwire` drop called.
        //
        // The `Tripwire` can move between tasks on the executor, which could cause
        // a stale waker pointing to the wrong task, preventing `Tripwire` from waking
        // up correctly.
        //
        // N.B. it's possible to check for this using the `Waker::will_wake`
        // function, but we omit that here to keep things simple.
        self.shared
            .wakers
            .lock()
            .expect("lock waker map success")
            .insert(self.id, cx.waker().clone());

        Poll::Pending
    }
}

impl Tripwire {
    #[must_use]
    /// Make a new `Tripwire` and an associated [`Trigger`].
    pub fn new() -> (Trigger, Tripwire) {
        let shared = Arc::new(Shared {
            wakers: Mutex::new(HashMap::default()),
            state: AtomicU64::new(0),
        });

        (
            Trigger {
                shared: Some(shared.clone()),
            },
            Tripwire {
                id: shared.next_id(),
                shared,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::time::{Duration, Instant};
    use tokio_stream::wrappers::ReceiverStream;

    #[test]
    #[allow(clippy::redundant_clone)]
    fn tripwire_id() {
        let (_tr, tw) = Tripwire::new();
        assert_eq!(tw.id, 1);
        assert_eq!(tw.clone().id, 2);
    }

    #[test]
    fn state_operations() {
        let mut state = 0u64;

        assert!(!get_cancellation(state));
        state = set_state(state, CANCELLED);
        assert!(get_cancellation(state));

        assert_eq!(get_next_id(state), 0);

        state = inc_next_id(state);
        assert!(get_cancellation(state));
        assert_eq!(get_next_id(state), 1);

        state = inc_next_id(state);
        state = set_state(state, 0); // 0 for NOT_CANCELLED
        assert!(!get_cancellation(state));
        assert_eq!(get_next_id(state), 2);
    }

    macro_rules! assert_pending {
        ($var:expr) => {
            assert!(futures::poll!(&mut $var).is_pending());
        };
    }

    macro_rules! assert_ready {
        ($var:expr) => {
            assert!(futures::poll!(&mut $var).is_ready());
        };
    }

    #[tokio::test]
    async fn drop_and_not_tripwire() {
        let (tr, mut tw) = Tripwire::new();
        assert_pending!(tw);
        drop(tr);
        assert_ready!(tw);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn drop_tr_at_beginning() {
        let (tr, mut tw) = Tripwire::new();
        drop(tr);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn disable_tr_at_beginning() {
        let (tr, mut tw) = Tripwire::new();
        tr.disable();
        assert_pending!(tw);
    }

    #[tokio::test]
    async fn cancel_and_tripwire_resolved() {
        let (tr, mut tw) = Tripwire::new();
        assert_pending!(tw);
        tr.cancel();
        assert_ready!(tw);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn cloned_tripwire() {
        let (tr, mut tw1) = Tripwire::new();
        assert_pending!(tw1);
        let mut tw2 = tw1.clone();

        assert_pending!(tw1);
        assert_pending!(tw2);

        tr.cancel();

        assert!(tw1.shared.wakers.lock().unwrap().is_empty());

        assert_ready!(tw1);
        assert_ready!(tw2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn take_until_cancel() {
        let (tr, mut tw1) = Tripwire::new();
        assert_pending!(tw1);

        let (tx, rx) = tokio::sync::mpsc::channel::<i32>(1);
        let mut stream = ReceiverStream::new(rx).take_until(tw1);

        assert_pending!(stream.next());
        tx.send(1).await.expect("send success");
        assert_eq!(stream.next().await, Some(1));
        assert_pending!(stream.next());

        tr.cancel();
        assert!(tx.send(2).await.is_ok());
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn take_until_drop() {
        let (tr, mut tw1) = Tripwire::new();
        assert_pending!(tw1);

        let (tx, rx) = tokio::sync::mpsc::channel::<i32>(1);
        let mut stream = ReceiverStream::new(rx).take_until(tw1);

        assert_pending!(stream.next());
        tx.send(1).await.expect("send success");
        assert_eq!(stream.next().await, Some(1));
        assert_pending!(stream.next());

        drop(tr);
        assert!(tx.send(2).await.is_ok());
        assert_eq!(stream.next().await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn with_timeout() {
        let (tr, tw) = Tripwire::new();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            tr.cancel();
        });

        tokio::spawn(async move {
            let deadline = Instant::now() + Duration::from_secs(2);

            tokio::time::timeout_at(deadline.into(), tw)
                .await
                .expect("not timeout");
        })
        .await
        .unwrap();
    }
}
