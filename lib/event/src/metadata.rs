use finalize::{EventFinalizers, EventStatus};
use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use crate::{BatchNotifier, EventFinalizer};

#[allow(clippy::derive_partial_eq_without_eq)]
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Default, Deserialize, PartialOrd, PartialEq, Serialize)]
pub struct EventMetadata {
    #[serde(default, skip)]
    finalizers: EventFinalizers,
}

impl From<EventFinalizers> for EventMetadata {
    fn from(finalizers: EventFinalizers) -> Self {
        Self { finalizers }
    }
}

impl ByteSizeOf for EventMetadata {
    fn allocated_bytes(&self) -> usize {
        // Note we don't count the `str` here because it's allocated
        // somewhere else. We're just moving around the pointer, which
        // is already captured by `ByteSizeOf::size_of`
        self.finalizers.allocated_bytes()
    }
}

impl EventMetadata {
    /// Replace the finalizers array with the given one.
    #[must_use]
    pub fn with_finalizer(mut self, finalizer: EventFinalizer) -> Self {
        self.finalizers = EventFinalizers::new(finalizer);
        self
    }

    /// Replace the finalizer with a new one created from the given batch notifier
    #[must_use]
    pub fn with_batch_notifier(self, batch: &BatchNotifier) -> Self {
        self.with_finalizer(EventFinalizer::new(batch.clone()))
    }

    /// Replace the finalizer with a new one created from the given optional
    /// batch notifier
    #[must_use]
    pub fn with_batch_notifier_option(self, batch: &Option<BatchNotifier>) -> Self {
        match batch {
            Some(batch) => self.with_finalizer(EventFinalizer::new(batch.clone())),
            None => self,
        }
    }

    /// Merge the other `EventMetadata` into this.
    /// If a Datadog API key is not set in `self`, the one from `other` will be used
    pub fn merge(&mut self, other: Self) {
        self.finalizers.merge(other.finalizers);
    }

    /// Update the finalizer(s) status
    pub fn update_status(&self, status: EventStatus) {
        self.finalizers.update_status(status);
    }

    /// Update the finalizer's sources
    pub fn update_sources(&mut self) {
        self.finalizers.update_sources();
    }

    /// Add a new finalizer to the array
    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.finalizers.add(finalizer);
    }

    /// Swap the finalizers list with an empty list and return the original
    pub fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }

    /// Merges the given finalizers into the existing set of finalizers.
    pub fn merge_finalizers(&mut self, finalizers: EventFinalizers) {
        self.finalizers.merge(finalizers);
    }
}

// TODO: impl EventDataEq ?

/// This is a simple wrapper to allow attaching `EventMetadata` to
/// any other type. This is primarily used in conversion functions,
/// such as `impl From<X> for WithMetadata<Y>`.
pub struct WithMetadata<T> {
    /// The data item being wrapped
    pub data: T,
    /// The additional metadata sidecar
    pub metadata: EventMetadata,
}

impl<T> WithMetadata<T> {
    /// Covert from one wrapped type to another, where the underlying
    /// type allows direct conversion.
    pub fn into<T1: From<T>>(self) -> WithMetadata<T1> {
        // We would like to `impl From` instead, but this fails due to
        // conflicting implementations of `impl<T> From<T> for T`.
        WithMetadata {
            data: T1::from(self.data),
            metadata: self.metadata,
        }
    }
}
