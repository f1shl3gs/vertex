use std::future::Future;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use crossbeam_utils::atomic::AtomicCell;
use futures::FutureExt;
use measurable::ByteSizeOf;
use pin_project_lite::pin_project;
use tokio::sync::oneshot;
use tracing::error;

/// The status of an individual event
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EventStatus {
    /// All copies of this event were dropped without being finalized
    /// the default.
    Dropped,
    /// All copies of this event were delivered successfully.
    Delivered,
    /// At least one copy of this event encountered a retriable error.
    Errored,
    /// At least one copy of this event encountered a permanent failure
    /// or rejection.
    Rejected,
    /// This status has been recorded and should not be updated
    Recorded,
}

impl EventStatus {
    /// Update this status with another event's finalization status and
    /// return the result.
    ///
    /// # Panics
    ///
    /// Passing a new status of `Dropped` is a programming error and
    /// will panic in debug/test builds.
    #[must_use]
    pub fn update(self, status: Self) -> Self {
        match (self, status) {
            // `Recorded` always overwrites existing status and is never updated
            (_, Self::Recorded) | (Self::Recorded, _) => Self::Recorded,
            // `Dropped` always updates to the new status,
            (Self::Dropped, _) => status,
            // Updates *to* `Dropped` are nonsense.
            (_, Self::Dropped) => {
                debug_assert!(false, "Updating EventStatus to Dropped is nonsense");
                self
            }
            // `Failed` overrides `Errored` or `Delivered`,
            (Self::Rejected, _) | (_, Self::Rejected) => Self::Rejected,
            // `Errored` overrides `Delivered`
            (Self::Errored, _) | (_, Self::Errored) => Self::Errored,
            // No change for `Delivered`
            (Self::Delivered, Self::Delivered) => Self::Delivered,
        }
    }
}

/// An object to which we can add a batch notifier.
pub trait AddBatchNotifier {
    /// Adds a single shared batch notifier to this type.
    fn add_batch_notifier(&mut self, notifier: BatchNotifier);
}

/// An object that can be finalized.
pub trait Finalizable {
    /// Consumes the finalizers of this Object.
    ///
    /// Typically used for coalescing the finalizers of multiple items,
    /// such as when batching finalizable objects where all finalizations
    /// will be processed when the batch itself is processed.
    fn take_finalizers(&mut self) -> EventFinalizers;
}

impl<T: Finalizable> Finalizable for Vec<T> {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.iter_mut()
            .fold(EventFinalizers::default(), |mut acc, x| {
                acc.merge(x.take_finalizers());
                acc
            })
    }
}

/// Wrapper type for an array of event finalizers. This is the primary public
/// interface to event finalization metadata.
#[derive(Clone, Debug, Default)]
pub struct EventFinalizers(Vec<Arc<EventFinalizer>>);

impl Eq for EventFinalizers {}

impl PartialEq for EventFinalizers {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len()
            && (self.0.iter())
                .zip(other.0.iter())
                .all(|(a, b)| Arc::ptr_eq(a, b))
    }
}

impl PartialOrd for EventFinalizers {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // There is no partial order defined structurally on `EventFinalizer`.
        // Partial equality is defined on the equality of `Arc`s. Therefore,
        // Partial ordering of `EventFinalizers` is defined only on the
        // length of the finalizers.
        self.0.len().partial_cmp(&other.0.len())
    }
}

impl ByteSizeOf for EventFinalizers {
    fn allocated_bytes(&self) -> usize {
        self.0.iter().fold(0, |acc, arc| acc + arc.size_of())
    }
}

impl Finalizable for EventFinalizers {
    fn take_finalizers(&mut self) -> EventFinalizers {
        mem::take(self)
    }
}

impl FromIterator<EventFinalizers> for EventFinalizers {
    fn from_iter<T: IntoIterator<Item = EventFinalizers>>(iter: T) -> Self {
        Self(iter.into_iter().flat_map(|f| f.0.into_iter()).collect())
    }
}

impl EventFinalizers {
    /// Default empty finalizer set for use in `const` contexts.
    pub const DEFAULT: Self = Self(Vec::new());

    /// Create a new array of event finalizer with the single event.
    pub fn new(finalizer: EventFinalizer) -> Self {
        Self(vec![Arc::new(finalizer)])
    }

    /// Add a single finalizer to this array.
    pub fn add(&mut self, finalizer: EventFinalizer) {
        self.0.push(Arc::new(finalizer));
    }

    /// Returns the number of event finalizers in the collection.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the collection contains no event finalizers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Merges the event finalizers from `other` into the collection.
    pub fn merge(&mut self, other: Self) {
        if self.0.is_empty() {
            self.0 = other.0;
        } else {
            self.0.extend(other.0);
            self.0.dedup_by(|a, b| Arc::ptr_eq(a, b));
        }
    }

    /// Update the status of all finalizers in this set.
    pub fn update_status(&self, status: EventStatus) {
        for finalizer in &self.0 {
            finalizer.update_status(status);
        }
    }

    /// Update all sources for this finalizer with the current status. This
    /// *drops* the finalizer array elements so they may imediately signal
    /// the source batch
    pub fn update_sources(&mut self) {
        let finalizers = mem::take(&mut self.0);
        for finalizer in &finalizers {
            finalizer.update_batch();
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BatchStatus {
    /// All events in the batch were accepted (the default)
    Delivered,
    /// At least one event in the batch had a transient error in delivery
    Errored,
    /// At least one event in the batch had a permanent failure.
    Failed,
}

impl BatchStatus {
    /// Update this status with another batch's delivery status, and
    /// return the result.
    fn update(self, status: EventStatus) -> Self {
        match (self, status) {
            // `Dropped` and `Delivered` do not change the status.
            (_, EventStatus::Dropped | EventStatus::Delivered) => self,
            // `Failed` overrides `Errored` and `Delivered`
            (Self::Failed, _) | (_, EventStatus::Rejected) => Self::Failed,
            // `Errored` overrides `Delivered`
            (Self::Errored, _) | (_, EventStatus::Errored) => Self::Errored,
            // No change for `Delivered`
            _ => self,
        }
    }
}

/// The non-shared data underlying the shared `BatchNotifier`
#[derive(Debug)]
struct OwnedBatchNotifier {
    status: AtomicCell<BatchStatus>,
    notifier: Option<oneshot::Sender<BatchStatus>>,
}

impl OwnedBatchNotifier {
    /// Sends the status of the notifier back to the source.
    fn send_status(&mut self) {
        if let Some(notifier) = self.notifier.take() {
            let status = self.status.load();
            // Ignore the error case, as it will happen during normal
            // source shutdown and we can't detect that here.
            let _ = notifier.send(status);
        }
    }
}

impl Drop for OwnedBatchNotifier {
    fn drop(&mut self) {
        self.send_status();
    }
}

/// A batch notifier contains the status of the current batch along
/// with a one-shot notifier to send that status back to the source.
/// It is shared among all events of a batch
#[derive(Clone, Debug)]
pub struct BatchNotifier(Arc<OwnedBatchNotifier>);

impl BatchNotifier {
    /// Create a new `BatchNotifier` along with the receiver
    /// used to await its finalization status.
    #[must_use]
    pub fn new_with_receiver() -> (Self, BatchStatusReceiver) {
        let (sender, receiver) = oneshot::channel();
        let notifier = OwnedBatchNotifier {
            status: AtomicCell::new(BatchStatus::Delivered),
            notifier: Some(sender),
        };

        (Self(Arc::new(notifier)), BatchStatusReceiver { receiver })
    }

    /// Optionally call `new_with_receiver` and wrap the result in `Option`s
    #[must_use]
    pub fn maybe_new_with_receiver(enabled: bool) -> (Option<Self>, Option<BatchStatusReceiver>) {
        if enabled {
            let (batch, receiver) = Self::new_with_receiver();
            (Some(batch), Some(receiver))
        } else {
            (None, None)
        }
    }

    /// Apply a new batch notifier to a batch of events, and returns
    /// the receiver.
    pub fn maybe_apply_to<T: AddBatchNotifier>(
        enabled: bool,
        events: &mut [T],
    ) -> Option<BatchStatusReceiver> {
        enabled.then(|| {
            let (batch, receiver) = Self::new_with_receiver();
            for event in events {
                event.add_batch_notifier(batch.clone());
            }

            receiver
        })
    }

    /// Update this notifier's status from the status of a finalized event.
    fn update_status(&self, status: EventStatus) {
        // The status starts as Delivered and can only change if the new status
        // is different than that.
        if status != EventStatus::Delivered && status != EventStatus::Dropped {
            self.0
                .status
                .fetch_update(|old_status| Some(old_status.update(status)))
                .unwrap_or_else(|_| unreachable!());
        }
    }
}

/// An event finalizer is the shared data required to handle tracking
/// the status of an event, and updating the status of a batch with
/// that when the event is dropped.
#[derive(Debug)]
pub struct EventFinalizer {
    status: AtomicCell<EventStatus>,
    batch: BatchNotifier,
}

impl ByteSizeOf for EventFinalizer {
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl EventFinalizer {
    /// Create a new event in a batch
    pub fn new(batch: BatchNotifier) -> Self {
        let status = AtomicCell::new(EventStatus::Dropped);
        Self { status, batch }
    }

    /// Update this finalizer's status in place with the given `EventStatus`
    pub fn update_status(&self, status: EventStatus) {
        self.status
            .fetch_update(|old_status| Some(old_status.update(status)))
            .unwrap_or_else(|_| unreachable!());
    }

    /// Update the batch for this event with this finalizer's status, and
    /// mark this eent as no longer requiring update.
    pub fn update_batch(&self) {
        let status = self
            .status
            .fetch_update(|_| Some(EventStatus::Recorded))
            .unwrap_or_else(|_| unreachable!());

        self.batch.update_status(status);
    }
}

impl Drop for EventFinalizer {
    fn drop(&mut self) {
        self.update_batch();
    }
}

pin_project! {
    /// A convenience new type wrapper for the one-shot receiver for
    /// an individual batch status.
    pub struct BatchStatusReceiver {
        receiver: oneshot::Receiver<BatchStatus>,
    }
}

impl Future for BatchStatusReceiver {
    type Output = BatchStatus;
    fn poll(mut self: Pin<&mut Self>, ctx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match self.receiver.poll_unpin(ctx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(status)) => Poll::Ready(status),
            Poll::Ready(Err(err)) => {
                error!(
                    message = "Batch status receiver dropped before sending.",
                    %err,
                );

                Poll::Ready(BatchStatus::Errored)
            }
        }
    }
}

impl BatchStatusReceiver {
    /// Wrapper for the underlying `try_recv` function.
    ///
    /// # Errors
    ///
    /// - `TryRecvError::Empty` if no value has been sent yet.
    /// - `TryRecvError::Closed` if the sender has dropped without sending a value
    pub fn try_recv(&mut self) -> Result<BatchStatus, oneshot::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::oneshot::error::TryRecvError::Empty;

    fn make_finalizer() -> (EventFinalizers, BatchStatusReceiver) {
        let (batch, receiver) = BatchNotifier::new_with_receiver();
        let finalizers = EventFinalizers::new(EventFinalizer::new(batch));
        assert_eq!(finalizers.len(), 1);
        (finalizers, receiver)
    }

    #[test]
    fn defaults() {
        let finalizer = EventFinalizers::default();
        assert_eq!(finalizer.len(), 0);
    }

    #[test]
    fn sends_notification() {
        let (fin, mut receiver) = make_finalizer();
        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(fin);
        assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
    }

    #[test]
    fn early_update() {
        let (mut fin, mut receiver) = make_finalizer();
        fin.update_status(EventStatus::Rejected);
        assert_eq!(receiver.try_recv(), Err(Empty));
        fin.update_sources();
        assert_eq!(fin.len(), 0);
        assert_eq!(receiver.try_recv(), Ok(BatchStatus::Failed));
    }

    #[test]
    fn clone_events() {
        let (fin1, mut receiver) = make_finalizer();
        let fin2 = fin1.clone();

        assert_eq!(fin1.len(), 1);
        assert_eq!(fin2.len(), 1);
        assert_eq!(fin1, fin2);

        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(fin1);
        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(fin2);
        assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
    }

    #[test]
    fn merge_events() {
        let mut fin0 = EventFinalizers::default();
        let (fin1, mut receiver1) = make_finalizer();
        let (fin2, mut receiver2) = make_finalizer();

        assert_eq!(fin0.len(), 0);
        fin0.merge(fin1);
        assert_eq!(fin0.len(), 1);
        fin0.merge(fin2);
        assert_eq!(fin0.len(), 2);

        assert_eq!(receiver1.try_recv(), Err(Empty));
        assert_eq!(receiver2.try_recv(), Err(Empty));
        drop(fin0);
        assert_eq!(receiver1.try_recv(), Ok(BatchStatus::Delivered));
        assert_eq!(receiver2.try_recv(), Ok(BatchStatus::Delivered));
    }

    #[test]
    fn clone_and_merge_events() {
        let (mut fin1, mut receiver) = make_finalizer();
        let fin2 = fin1.clone();

        fin1.merge(fin2);
        assert_eq!(fin1.len(), 1);

        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(fin1);
        assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
    }

    #[test]
    fn multi_event_batch() {
        let (batch, mut receiver) = BatchNotifier::new_with_receiver();
        let event1 = EventFinalizers::new(EventFinalizer::new(batch.clone()));
        let mut event2 = EventFinalizers::new(EventFinalizer::new(batch.clone()));
        let event3 = EventFinalizers::new(EventFinalizer::new(batch.clone()));
        let event4 = event1.clone();

        drop(batch);
        assert_eq!(event1.len(), 1);
        assert_eq!(event2.len(), 1);
        assert_eq!(event3.len(), 1);
        assert_eq!(event4.len(), 1);
        assert_ne!(event1, event2);
        assert_ne!(event1, event3);
        assert_eq!(event1, event4);
        assert_ne!(event2, event3);
        assert_ne!(event2, event4);
        assert_ne!(event3, event4);

        // and merge another
        event2.merge(event3);
        assert_eq!(event2.len(), 2);
        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(event1);
        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(event2);
        assert_eq!(receiver.try_recv(), Err(Empty));
        drop(event4);
        assert_eq!(receiver.try_recv(), Ok(BatchStatus::Delivered));
    }

    #[test]
    fn event_status_updates() {
        use EventStatus::{Delivered, Dropped, Errored, Recorded, Rejected};

        assert_eq!(Dropped.update(Dropped), Dropped);
        assert_eq!(Dropped.update(Delivered), Delivered);
        assert_eq!(Dropped.update(Errored), Errored);
        assert_eq!(Dropped.update(Rejected), Rejected);
        assert_eq!(Dropped.update(Recorded), Recorded);

        assert_eq!(Delivered.update(Delivered), Delivered);
        assert_eq!(Delivered.update(Errored), Errored);
        assert_eq!(Delivered.update(Rejected), Rejected);
        assert_eq!(Delivered.update(Recorded), Recorded);

        assert_eq!(Errored.update(Delivered), Errored);
        assert_eq!(Errored.update(Errored), Errored);
        assert_eq!(Errored.update(Rejected), Rejected);
        assert_eq!(Errored.update(Recorded), Recorded);

        assert_eq!(Rejected.update(Delivered), Rejected);
        assert_eq!(Rejected.update(Errored), Rejected);
        assert_eq!(Rejected.update(Rejected), Rejected);
        assert_eq!(Rejected.update(Recorded), Recorded);

        assert_eq!(Recorded.update(Delivered), Recorded);
        assert_eq!(Recorded.update(Errored), Recorded);
        assert_eq!(Recorded.update(Rejected), Recorded);
        assert_eq!(Recorded.update(Recorded), Recorded);
    }

    #[test]
    fn batch_status_update() {
        use BatchStatus::{Delivered, Errored, Failed};

        assert_eq!(Delivered.update(EventStatus::Dropped), Delivered);
        assert_eq!(Delivered.update(EventStatus::Delivered), Delivered);
        assert_eq!(Delivered.update(EventStatus::Errored), Errored);
        assert_eq!(Delivered.update(EventStatus::Rejected), Failed);
        assert_eq!(Delivered.update(EventStatus::Recorded), Delivered);

        assert_eq!(Errored.update(EventStatus::Dropped), Errored);
        assert_eq!(Errored.update(EventStatus::Delivered), Errored);
        assert_eq!(Errored.update(EventStatus::Errored), Errored);
        assert_eq!(Errored.update(EventStatus::Rejected), Failed);
        assert_eq!(Errored.update(EventStatus::Recorded), Errored);

        assert_eq!(Failed.update(EventStatus::Dropped), Failed);
        assert_eq!(Failed.update(EventStatus::Delivered), Failed);
        assert_eq!(Failed.update(EventStatus::Errored), Failed);
        assert_eq!(Failed.update(EventStatus::Rejected), Failed);
        assert_eq!(Failed.update(EventStatus::Recorded), Failed);
    }
}
