//! A state wrapper that delays deletes.

use std::collections::VecDeque;
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::{sleep_until, timeout_at, Instant};
use tonic::codegen::BoxFuture;

/// A `super::Write` implementation that wraps another `super::Write` and
/// delays the delete calls.
/// Implements the logic for delaying the deletion of items from the storage.
pub struct Writer<T>
where
    T: super::Write,
    <T as super::Write>::Item: Send + Sync,
{
    inner: T,
    queue: VecDeque<(<T as super::Write>::Item, Instant)>,
    sleep: Duration,
}

impl<T> Writer<T>
where
    T: super::Write,
    <T as super::Write>::Item: Send + Sync,
{
    /// Take a `super::Write` and return it wrapped with `Writer`
    pub fn new(inner: T, sleep: Duration) -> Self {
        let queue = VecDeque::new();

        Self {
            inner,
            queue,
            sleep,
        }
    }
}

impl<T> Writer<T>
where
    T: super::Write,
    <T as super::Write>::Item: Send + Sync,
{
    /// Schedules the delayed deletion of the item at the future.
    pub fn schedule_delete(&mut self, item: <T as super::Write>::Item) {
        let deadline = Instant::now() + self.sleep;
        self.queue.push_back((item, deadline));
    }

    /// Clear the delayed deletion requests.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Perform the queued deletions.
    pub async fn perform(&mut self) {
        let now = Instant::now();

        while let Some((_, deadline)) = self.queue.front() {
            let deadline = *deadline;

            trace!(message = "Got delayed deletion deadline", ?deadline, ?now);
            if deadline > now {
                break;
            }

            trace!(
                message = "Processing delayed deletion for deadline",
                ?deadline,
                ?now
            );

            let (item, _) = self.queue.pop_front().unwrap();
            self.inner.delete(item).await;
        }
    }

    /// Obtain the next deadline.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.queue.front().map(|(_, instant)| *instant)
    }
}

#[async_trait]
impl<T> super::Write for Writer<T>
where
    T: super::Write + Send,
    <T as super::Write>::Item: Send + Sync,
{
    type Item = <T as super::Write>::Item;

    async fn add(&mut self, item: Self::Item) {
        self.inner.add(item).await
    }

    async fn update(&mut self, item: Self::Item) {
        self.inner.update(item).await
    }

    async fn delete(&mut self, item: Self::Item) {
        let deadline = Instant::now() + self.sleep;
        self.queue.push_back((item, deadline));
    }

    async fn resync(&mut self) {
        self.queue.clear();
        self.inner.resync().await
    }
}

#[async_trait]
impl<T> super::MaintainedWrite for Writer<T>
where
    T: super::Write,
    <T as super::Write>::Item: Send + Sync,
{
    fn maintenance_request(&mut self) -> Option<BoxFuture<'_, ()>> {
        let delayed_delete_deadline = self.next_deadline();
        let downstream = self.inner.maintenance_request();

        match (downstream, delayed_delete_deadline) {
            (Some(downstream), Some(delayed_delete_deadline)) => {
                let fut = timeout_at(delayed_delete_deadline, downstream).map(|_| ());
                Some(Box::pin(fut))
            }
            (None, Some(delayed_delete_deadline)) => {
                Some(Box::pin(sleep_until(delayed_delete_deadline)))
            }
            (Some(downstream), None) => Some(downstream),
            (None, None) => None,
        }
    }

    async fn perform_maintenance(&mut self) {
        // Perform delayed deletes.
        self.perform().awiat;

        // Do the downstream maintenance
        self.inner.perform_maintenance().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO
}
