#![deny(warnings)]
#![deny(unused_qualifications)]
#![deny(clippy::all)]
#![allow(
    clippy::module_name_repetitions,
    clippy::uninlined_format_args,
    clippy::cast_lossless,
    clippy::must_use_candidate,
    clippy::cast_possible_wrap,
    clippy::redundant_closure_for_method_calls
)]

pub mod array;
mod buffer;
pub mod log;
mod metadata;
pub mod metric;
pub mod tags;
pub mod trace;

// re-export
pub use array::{EventContainer, Events};
pub use finalize::{
    AddBatchNotifier, BatchNotifier, BatchStatus, BatchStatusReceiver, EventFinalizer,
    EventFinalizers, EventStatus, Finalizable,
};
pub use log::{LogRecord, event_path};
pub use metadata::EventMetadata;
pub use metric::{Bucket, Kind, Metric, MetricSeries, MetricValue, Quantile};
pub use trace::Trace;

use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::Utc;
use log_schema::log_schema;
use serde::Serialize;
use typesize::TypeSize;

#[derive(PartialEq, Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Log(LogRecord),
    Metric(Metric),
    Trace(Trace),
}

impl AddBatchNotifier for Event {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        let finalizer = EventFinalizer::new(notifier);

        match self {
            Event::Log(log) => log.add_finalizer(finalizer),
            Event::Metric(metric) => metric.add_finalizer(finalizer),
            Event::Trace(trace) => trace.add_finalizer(finalizer),
        }
    }
}

impl TypeSize for Event {
    fn allocated_bytes(&self) -> usize {
        match self {
            Event::Log(log) => log.allocated_bytes(),
            Event::Metric(metric) => metric.allocated_bytes(),
            Event::Trace(trace) => trace.allocated_bytes(),
        }
    }
}

impl Finalizable for Event {
    fn take_finalizers(&mut self) -> EventFinalizers {
        match self {
            Event::Log(log) => log.take_finalizers(),
            Event::Metric(metric) => metric.take_finalizers(),
            Event::Trace(span) => span.take_finalizers(),
        }
    }
}

impl EventDataEq for Event {
    fn event_data_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Log(a), Self::Log(b)) => a.event_data_eq(b),
            (Self::Metric(a), Self::Metric(b)) => a.event_data_eq(b),
            _ => false,
        }
    }
}

impl Event {
    /// Return self as a mutable `Metric`
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than an `Event::Metric`.
    pub fn as_mut_metric(&mut self) -> &mut Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self),
        }
    }

    /// Return self as a `Metric`
    ///
    /// # Panics
    ///
    /// This function panics if self is self is anything other than an `Event::Metric`.
    pub fn as_metric(&self) -> &Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self),
        }
    }

    /// Coerces self into `Metric`
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than an `Event::Metric`.
    pub fn into_metric(self) -> Metric {
        match self {
            Event::Metric(m) => m,
            _ => panic!("Failed type coercion, {:?} is not a metric", self),
        }
    }

    /// Coerces self into a `LogRecord`
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than an `Event::Log`
    pub fn into_log(self) -> LogRecord {
        match self {
            Event::Log(log) => log,
            _ => panic!("Failed type coercion, {:?} is not a log event", self),
        }
    }

    /// Return self as a `LogRecord`
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than an `Event::Log`
    pub fn as_log(&self) -> &LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    /// Return self as a mutable `LogRecord`
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than an `Event::Log`.
    pub fn as_mut_log(&mut self) -> &mut LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    /// Return self as a `Trace`.
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than a `Event::Trace`
    pub fn as_trace(&self) -> &Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a trace", self),
        }
    }

    /// Return self as a `Trace`.
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than a `Event::Trace`
    pub fn into_trace(self) -> Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a trace", self),
        }
    }

    /// Return self as a mutable `Trace`.
    ///
    /// # Panics
    ///
    /// This function panics if self is anything other than a `Event::Trace`
    pub fn as_mut_trace(&mut self) -> &mut Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Log(log) => log.metadata(),
            Self::Metric(metric) => metric.metadata(),
            Self::Trace(span) => span.metadata(),
        }
    }

    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        match self {
            Self::Metric(metric) => metric.metadata_mut(),
            Self::Log(log) => log.metadata_mut(),
            Self::Trace(span) => span.metadata_mut(),
        }
    }

    pub fn add_batch_notifier(&mut self, batch: BatchNotifier) {
        let finalizer = EventFinalizer::new(batch);
        match self {
            Self::Log(log) => log.add_finalizer(finalizer),
            Self::Metric(metric) => metric.add_finalizer(finalizer),
            Self::Trace(span) => span.add_finalizer(finalizer),
        }
    }

    #[must_use]
    pub fn with_batch_notifier(self, batch: &BatchNotifier) -> Self {
        match self {
            Self::Log(log) => log.with_batch_notifier(batch).into(),
            Self::Metric(metric) => metric.with_batch_notifier(batch).into(),
            Self::Trace(span) => span.with_batch_notifier(batch).into(),
        }
    }

    /// Replace the finalizer with a new one created from the given optional
    /// batch notifier
    #[must_use]
    pub fn with_batch_notifier_option(self, batch: &Option<BatchNotifier>) -> Self {
        match self {
            Self::Log(log) => log.with_batch_notifier_option(batch).into(),
            Self::Metric(metric) => metric.with_batch_notifier_option(batch).into(),
            Self::Trace(span) => span.with_batch_notifier_option(batch).into(),
        }
    }
}

impl Event {
    /// Returns the in-memory size of this type
    pub fn size_of(&self) -> usize {
        size_of_val(self) + self.allocated_bytes()
    }

    /// Returns the allocated bytes of this type
    pub fn allocated_bytes(&self) -> usize {
        match self {
            Event::Metric(metric) => metric.allocated_bytes(),
            Event::Log(log) => log.allocated_bytes(),
            Event::Trace(span) => span.allocated_bytes(),
        }
    }
}

impl From<Metric> for Event {
    fn from(m: Metric) -> Self {
        Self::Metric(m)
    }
}

impl From<LogRecord> for Event {
    fn from(r: LogRecord) -> Self {
        Self::Log(r)
    }
}

impl From<Trace> for Event {
    fn from(trace: Trace) -> Self {
        Self::Trace(trace)
    }
}

impl From<BTreeMap<String, log::Value>> for Event {
    fn from(m: BTreeMap<String, log::Value>) -> Self {
        Self::Log(m.into())
    }
}

impl From<value::Value> for Event {
    fn from(value: value::Value) -> Self {
        Event::Log(value.into())
    }
}

impl From<String> for Event {
    fn from(s: String) -> Self {
        let mut fields: BTreeMap<String, log::Value> = BTreeMap::new();
        fields.insert("message".to_string(), log::Value::Bytes(s.into()));
        fields.insert(
            log_schema().timestamp_key().to_string(),
            log::Value::Timestamp(Utc::now()),
        );

        Self::Log(fields.into())
    }
}

impl From<Bytes> for Event {
    fn from(b: Bytes) -> Self {
        let log = LogRecord::from(b);
        log.into()
    }
}

impl From<&str> for Event {
    fn from(s: &str) -> Self {
        let log = LogRecord::from(s);
        Self::Log(log)
    }
}

/// A wrapper for references to inner event types, where reconstituting
/// a full `Event` from a `LogRecord`, `Metric` or `Span` might be inconvenient.
#[derive(Clone, Copy, Debug)]
pub enum EventRef<'a> {
    Log(&'a LogRecord),
    Metric(&'a Metric),
    Trace(&'a Trace),
}

impl<'a> EventRef<'a> {
    /// Extract the `LogRecord` reference in this.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `LogRecord` reference.
    pub fn as_log(self) -> &'a LogRecord {
        match self {
            Self::Log(log) => log,
            _ => panic!("Failed type coercion, {:?} is not a log reference", self),
        }
    }

    /// Convert this reference into a new `LogRecord` by cloning.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `LogRecord` reference.
    pub fn into_log(self) -> LogRecord {
        match self {
            Self::Log(log) => log.clone(),
            _ => panic!("Failed type coercion, {:?} is not a log reference", self),
        }
    }

    /// Convert this reference into a new `Metric` by cloning
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `Metric` reference.
    pub fn into_metric(self) -> Metric {
        match self {
            Self::Metric(metric) => metric.clone(),
            _ => panic!("Failed type coercion, {:?} is not a metric reference", self),
        }
    }
}

impl<'a> From<&'a Event> for EventRef<'a> {
    fn from(event: &'a Event) -> Self {
        match event {
            Event::Log(log) => log.into(),
            Event::Metric(metric) => metric.into(),
            Event::Trace(span) => span.into(),
        }
    }
}

impl<'a> From<&'a LogRecord> for EventRef<'a> {
    fn from(log: &'a LogRecord) -> Self {
        Self::Log(log)
    }
}

impl<'a> From<&'a Metric> for EventRef<'a> {
    fn from(metric: &'a Metric) -> Self {
        Self::Metric(metric)
    }
}

impl<'a> From<&'a Trace> for EventRef<'a> {
    fn from(trace: &'a Trace) -> Self {
        Self::Trace(trace)
    }
}

impl EventDataEq<Event> for EventRef<'_> {
    fn event_data_eq(&self, other: &Event) -> bool {
        match (self, other) {
            (Self::Log(a), Event::Log(b)) => a.event_data_eq(b),
            (Self::Metric(a), Event::Metric(b)) => a.event_data_eq(b),
            (Self::Trace(a), Event::Trace(b)) => a.event_data_eq(b),
            _ => false,
        }
    }
}

impl TryInto<serde_json::Value> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        match self {
            Event::Log(log) => serde_json::to_value(log),
            Event::Metric(metric) => serde_json::to_value(metric),
            Event::Trace(trace) => serde_json::to_value(trace),
        }
    }
}

/// TODO: Share this Error type
/// Vertex's basic error type, dynamically dispatched and safe to send across
/// threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl TryFrom<serde_json::Value> for Event {
    type Error = Error;

    fn try_from(map: serde_json::Value) -> Result<Self, Self::Error> {
        match map {
            serde_json::Value::Object(fields) => Ok(Event::from(
                fields
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<BTreeMap<_, _>>(),
            )),
            _ => Err(Error::from(
                "Attempted to convert non-Object JSON into an Event",
            )),
        }
    }
}

pub trait MaybeAsLogMut {
    fn maybe_as_log_mut(&mut self) -> Option<&mut LogRecord>;
}

impl MaybeAsLogMut for Event {
    fn maybe_as_log_mut(&mut self) -> Option<&mut LogRecord> {
        match self {
            Event::Log(log) => Some(log),
            _ => None,
        }
    }
}

/// A related trait to `PartialEq`, `EventDataEq` tests if two events
/// contain the same data, exclusive of the metadata. This is used to
/// test for events having the same values but potentially different
/// parts of the metadata that not fixed between runs, without removing
/// the ability to compare them for exact equality.
pub trait EventDataEq<Rhs: ?Sized = Self> {
    fn event_data_eq(&self, other: &Rhs) -> bool;
}

impl<T: EventDataEq> EventDataEq for &[T] {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other.iter())
                .all(|(a, b)| a.event_data_eq(b))
    }
}

impl<T: EventDataEq> EventDataEq for Vec<T> {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.as_slice().event_data_eq(&other.as_slice())
    }
}
