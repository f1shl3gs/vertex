use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use tracing::info;

struct Shared {
    name: String,

    closed: AtomicBool,
    cancelled: AtomicBool,

    waited: AtomicU64,
    wakers: Mutex<HashMap<u64, Waker>>,
}

impl Shared {
    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn next_id(&self) -> u64 {
        self.waited.fetch_add(1, Ordering::SeqCst)
    }
}

pub struct Trigger {
    shared: Arc<Shared>,
}

impl Trigger {
    /// Cancel all associated tripwire, make them immediately resolved.
    pub fn cancel(mut self) {
        info!("{} cancel trigger", self.shared.name);

        self.shared.closed.store(true, Ordering::SeqCst);
        self.shared.cancelled.store(true, Ordering::SeqCst);

        self.wake_all();
    }

    pub fn disable(self) {
        info!("{} disable trigger", self.shared.name);

        self.shared.closed.store(true, Ordering::SeqCst);
    }

    fn wake_all(&mut self) {
        self.shared
            .wakers
            .lock()
            .expect("lock waker map success")
            .drain()
            .into_iter()
            .for_each(|(key, waker)| {
                info!("{} wake {}", self.shared.name, key);

                waker.wake()
            });
    }
}

impl Drop for Trigger {
    fn drop(&mut self) {
        info!("{} trigger dropped", self.shared.name);

        self.shared.closed.store(true, Ordering::SeqCst);
        self.wake_all();
    }
}

pub struct Tripwire {
    shared: Arc<Shared>,
    id: u64,
}

impl Clone for Tripwire {
    fn clone(&self) -> Self {
        let shared = self.shared.clone();
        let id = shared.next_id();

        Self { shared, id }
    }
}

impl Drop for Tripwire {
    fn drop(&mut self) {
        info!("{} tripwire drop", self.shared.name);

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
        let cancelled = self.shared.cancelled();

        info!("{} {} polled {}", self.shared.name, self.id, cancelled);

        if cancelled {
            info!("{} {} ready", self.shared.name, self.id);

            return Poll::Ready(());
        }

        if self.shared.closed.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        self.shared
            .wakers
            .lock()
            .expect("lock waker map success")
            .insert(self.id, cx.waker().clone());

        info!("{} {} pending", self.shared.name, self.id);

        Poll::Pending
    }
}

impl Tripwire {
    pub fn new(name: impl Into<String>) -> (Trigger, Tripwire) {
        let shared = Arc::new(Shared {
            name: name.into(),
            closed: AtomicBool::new(false),
            wakers: Mutex::new(Default::default()),
            cancelled: AtomicBool::new(false),
            waited: AtomicU64::new(0),
        });

        (
            Trigger {
                shared: shared.clone(),
            },
            Tripwire {
                id: shared.next_id(),
                shared,
            },
        )
    }

    pub fn closed(&self) -> bool {
        self.shared.closed.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::time::{Duration, Instant};
    use tokio_stream::wrappers::ReceiverStream;

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
        let (tr, mut tw) = Tripwire::new("");
        assert_pending!(tw);
        drop(tr);
        assert_ready!(tw);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn drop_tr_at_beginning() {
        let (tr, mut tw) = Tripwire::new("");
        drop(tr);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn disable_tr_at_beginning() {
        let (tr, mut tw) = Tripwire::new("");
        tr.disable();
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn cancel_and_tripwire_resolved() {
        let (tr, mut tw) = Tripwire::new("");
        assert_pending!(tw);
        tr.cancel();
        assert_ready!(tw);
        assert_ready!(tw);
    }

    #[tokio::test]
    async fn cloned_tripwire() {
        let (tr, mut tw1) = Tripwire::new("");
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
        let (tr, mut tw1) = Tripwire::new("");
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
        let (tr, mut tw1) = Tripwire::new("");
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
        let (tr, tw) = Tripwire::new("");

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            tr.cancel();
        });

        let triggered = tokio::spawn(async move {
            let deadline = Instant::now() + Duration::from_secs(2);

            match tokio::time::timeout_at(deadline.into(), tw).await {
                Ok(()) => true,
                Err(_) => panic!("timeout"),
            }
        })
        .await
        .unwrap();

        assert!(triggered);
    }
}
