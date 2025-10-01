use std::borrow::Cow;
use std::collections::BTreeMap;

use finalize::{EventFinalizers, EventStatus};
use serde::{Deserialize, Serialize};
use typesize::TypeSize;
use value::Value;

use crate::{BatchNotifier, EventFinalizer};

fn default_metadata_value() -> Value {
    Value::Object(BTreeMap::new())
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventMetadata {
    #[serde(default, skip)]
    finalizers: EventFinalizers,

    /// Arbitrary data stored with an event.
    #[serde(default = "default_metadata_value")]
    pub(crate) value: Value,

    /// The id of the source.
    pub(crate) source_id: Option<Cow<'static, str>>,

    /// The type of the source.
    pub(crate) source_type: Option<Cow<'static, str>>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        Self {
            finalizers: EventFinalizers::default(),
            value: Value::Object(BTreeMap::new()),
            source_id: None,
            source_type: None,
        }
    }
}

impl From<EventFinalizers> for EventMetadata {
    fn from(finalizers: EventFinalizers) -> Self {
        Self {
            finalizers,
            value: default_metadata_value(),
            source_id: None,
            source_type: None,
        }
    }
}

impl TypeSize for EventMetadata {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl EventMetadata {
    /// Creates `EventMetadata` with the given `Value`, and the rest of the
    /// fields with default values.
    pub fn default_with_value(value: Value) -> Self {
        EventMetadata {
            finalizers: Default::default(),
            value,
            source_id: None,
            source_type: None,
        }
    }

    pub fn from_parts(
        value: Value,
        source_id: Option<Cow<'static, str>>,
        source_type: Option<Cow<'static, str>>,
    ) -> Self {
        Self {
            finalizers: Default::default(),
            value,
            source_id,
            source_type,
        }
    }

    /// Returns a reference to the metadata value.
    #[inline]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Returns a mutable reference to the metadata value.
    #[inline]
    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Returns a reference to the metadata source id.
    #[inline]
    pub fn source_id(&self) -> Option<&str> {
        self.source_id.as_deref()
    }

    /// Returns a reference to the metadata source type.
    #[inline]
    pub fn source_type(&self) -> Option<&str> {
        self.source_type.as_deref()
    }

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
