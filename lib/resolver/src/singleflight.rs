use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::mem::MaybeUninit;
use std::sync::{Arc, Mutex};

use tokio::sync::Notify;

#[derive(Debug)]
pub struct SingleFlight<K, T> {
    tasks: Mutex<HashMap<K, Arc<Call<T>>>>,
}

impl<K, T> SingleFlight<K, T>
where
    K: Clone + Eq + Hash,
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn call<F>(&self, key: K, fut: F) -> T
    where
        F: Future<Output = T>,
    {
        let (call, shared) = {
            let mut tasks = self.tasks.lock().unwrap();

            let mut shared = true;
            let call = tasks
                .entry(key.clone())
                .or_insert_with(|| {
                    shared = false;
                    Arc::new(Call::new())
                })
                .clone();
            drop(tasks);

            (call, shared)
        };

        if shared {
            call.notify.notified().await;
            unsafe { (*call.value.get()).assume_init_read() }
        } else {
            let result = fut.await;

            self.tasks.lock().unwrap().remove(&key).unwrap();

            call.set(result.clone());
            call.notify.notify_waiters();

            result
        }
    }
}

#[derive(Debug)]
struct Call<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    notify: Notify,
}

unsafe impl<T: Send> Send for Call<T> {}
unsafe impl<T: Send> Sync for Call<T> {}

impl<T> Call<T> {
    fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            notify: Notify::new(),
        }
    }

    #[inline]
    fn set(&self, value: T) {
        unsafe { (*self.value.get()).write(value) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::task::JoinSet;

    #[tokio::test]
    async fn single_flight() {
        static HITS: AtomicUsize = AtomicUsize::new(0);

        let group = Arc::new(SingleFlight::new());
        let mut tasks = JoinSet::new();

        for i in 0..10 {
            let group = Arc::clone(&group);

            tasks.spawn(async move {
                group
                    .call("foo", async {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        HITS.fetch_add(1, Ordering::SeqCst);
                        i
                    })
                    .await
            });
        }

        while let Some(Ok(value)) = tasks.join_next().await {
            assert_eq!(value, 0);
        }

        assert_eq!(HITS.load(Ordering::Acquire), 1);
    }
}
