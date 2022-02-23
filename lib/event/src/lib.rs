mod array;
pub mod encoding;
mod finalization;
pub mod log;
mod logfmt;
mod macros;
mod metadata;
mod metric;
mod proto;
pub mod trace;

// re-export
pub use array::EventContainer;
pub use buffers::{DecodeBytes, EncodeBytes};
pub use finalization::{
    BatchNotifier, BatchStatus, BatchStatusReceiver, EventFinalizer, EventFinalizers, EventStatus,
    Finalizable,
};
pub use log::value::Value;
pub use log::LogRecord;
pub use macros::EventDataEq;
pub use metadata::EventMetadata;
pub use metric::*;
pub use trace::{Trace, Traces};

use std::collections::btree_map;
use std::collections::BTreeMap;
use std::sync::Arc;

use bytes::{Buf, BufMut, Bytes};
use prost::Message;
use prost::{DecodeError, EncodeError};
use shared::ByteSizeOf;

use crate::log::Logs;
use crate::proto::EventWrapper;

#[derive(PartialEq, Debug, Clone)]
pub enum Event {
    Log(LogRecord),
    Metric(Metric),
    Trace(Trace),
}

/// An array of one of the `Event` variants exclusively
pub enum Events {
    /// An array of type `LogRecord`
    Logs(Logs),
    /// An array of type `Metric`
    Metrics(Metrics),
    /// An array of type `Trace`
    Traces(Traces),
}

impl From<Event> for Events {
    fn from(event: Event) -> Self {
        match event {
            Event::Log(log) => Self::Logs(vec![log]),
            Event::Metric(metric) => Self::Metrics(vec![metric]),
            Event::Trace(trace) => Self::Traces(vec![trace]),
        }
    }
}

impl ByteSizeOf for Events {
    fn allocated_bytes(&self) -> usize {
        match self {
            Self::Logs(logs) => logs.allocated_bytes(),
            Self::Metrics(metrics) => metrics.allocated_bytes(),
            Self::Traces(spans) => spans.allocated_bytes(),
        }
    }
}

impl ByteSizeOf for Event {
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
    /// This function panics if self is anything other than an `Event::Metric`
    pub fn as_mut_metric(&mut self) -> &mut Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self),
        }
    }

    pub fn as_metric(&self) -> &Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self),
        }
    }

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

    pub fn as_log(&self) -> &LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    pub fn as_mut_log(&mut self) -> &mut LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    pub fn as_trace(&self) -> &Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a trace", self),
        }
    }

    pub fn into_trace(self) -> Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a trace", self),
        }
    }

    pub fn as_mut_trace(&mut self) -> &mut Trace {
        match self {
            Event::Trace(trace) => trace,
            _ => panic!("Failed type coercion, {:?} is not a log", self),
        }
    }

    pub fn tags(&self) -> &BTreeMap<String, String> {
        match self {
            Event::Log(log) => &log.tags,
            Event::Metric(metric) => metric.tags(),
            Event::Trace(trace) => &trace.tags,
        }
    }

    pub fn tag_entry(&mut self, key: impl Into<String>) -> btree_map::Entry<String, String> {
        match self {
            Self::Log(log) => log.tags.entry(key.into()),
            Self::Metric(metric) => metric.series.tags.entry(key.into()),
            _ => unreachable!(),
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

    #[inline]
    pub fn new_empty_log() -> Self {
        Event::Log(LogRecord::default())
    }

    pub fn add_batch_notifier(&mut self, batch: Arc<BatchNotifier>) {
        let finalizer = EventFinalizer::new(batch);
        match self {
            Self::Log(log) => log.add_finalizer(finalizer),
            Self::Metric(metric) => metric.add_finalizer(finalizer),
            Self::Trace(span) => span.add_finalizer(finalizer),
        }
    }

    pub fn with_batch_notifier(self, batch: &Arc<BatchNotifier>) -> Self {
        match self {
            Self::Log(log) => log.with_batch_notifier(batch).into(),
            Self::Metric(metric) => metric.with_batch_notifier(batch).into(),
            Self::Trace(span) => span.with_batch_notifier(batch).into(),
        }
    }

    /// Replace the finalizer with a new one created from the given optional
    /// batch notifier
    pub fn with_batch_notifier_option(self, batch: &Option<Arc<BatchNotifier>>) -> Self {
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
        std::mem::size_of_val(self) + self.allocated_bytes()
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

impl From<BTreeMap<String, Value>> for Event {
    fn from(m: BTreeMap<String, Value>) -> Self {
        Self::Log(m.into())
    }
}

impl From<String> for Event {
    fn from(s: String) -> Self {
        let mut fields: BTreeMap<String, Value> = BTreeMap::new();
        fields.insert("message".to_string(), Value::Bytes(s.into()));

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

impl EncodeBytes for Event {
    type Error = EncodeError;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::Error>
    where
        B: BufMut,
        Self: Sized,
    {
        EventWrapper::from(self).encode(buffer)
    }
}

impl DecodeBytes for crate::Event {
    type Error = DecodeError;

    fn decode<B>(buffer: B) -> Result<Event, Self::Error>
    where
        Event: Sized,
        B: Buf,
    {
        EventWrapper::decode(buffer).map(Into::into)
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
/// Vector's basic error type, dynamically dispatched and safe to send across
/// threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl TryFrom<serde_json::Value> for Event {
    type Error = crate::Error;

    fn try_from(map: serde_json::Value) -> Result<Self, Self::Error> {
        match map {
            serde_json::Value::Object(fields) => Ok(Event::from(
                fields
                    .into_iter()
                    .map(|(k, v)| (k, v.into()))
                    .collect::<BTreeMap<_, _>>(),
            )),
            _ => Err(crate::Error::from(
                "Attempted to convert non-Object JSON into an Event.",
            )),
        }
    }
}
