pub mod encoding;
mod finalization;
mod log;
mod logfmt;
mod macros;
mod metadata;
mod metric;
mod trace;
mod value;
mod buffer;

// re-export
pub use finalization::*;
pub use log::LogRecord;
pub use metric::*;
pub use value::Value;
pub use buffer::{DecodeBytes, EncodeBytes};

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::finalization::{BatchNotifier, EventFinalizer};
use bytes::{Buf, BufMut};
use prost::{DecodeError, EncodeError};

pub trait ByteSizeOf {
    /// Returns the in-memory size of this type
    ///
    /// This function returns the total number of bytes that
    /// [`std::mem::size_of`] does in addition to any interior
    /// allocated bytes. It default implementation is `std::mem::size_of`
    /// + `ByteSizeOf::allocated_bytes`
    fn size_of(&self) -> usize {
        std::mem::size_of_val(self) + self.allocated_bytes()
    }

    /// Returns the allocated bytes of this type
    fn allocated_bytes(&self) -> usize;
}

macro_rules! impl_byte_size_of_for_num {
    ($typ:ident) => {
        impl ByteSizeOf for $typ {
            fn allocated_bytes(&self) -> usize {
                0
            }
        }
    };
}

impl_byte_size_of_for_num!(u8);
impl_byte_size_of_for_num!(u16);
impl_byte_size_of_for_num!(u32);
impl_byte_size_of_for_num!(u64);
impl_byte_size_of_for_num!(u128);
impl_byte_size_of_for_num!(i8);
impl_byte_size_of_for_num!(i16);
impl_byte_size_of_for_num!(i32);
impl_byte_size_of_for_num!(i64);
impl_byte_size_of_for_num!(i128);
impl_byte_size_of_for_num!(f32);
impl_byte_size_of_for_num!(f64);

impl ByteSizeOf for String {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl<K, V> ByteSizeOf for BTreeMap<K, V>
where
    K: ByteSizeOf,
    V: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.size_of() + v.size_of())
    }
}

impl<T> ByteSizeOf for Vec<T>
where
    T: ByteSizeOf,
{
    fn allocated_bytes(&self) -> usize {
        self.iter().fold(0, |acc, i| acc + i.size_of())
    }
}

#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Event {
    Log(LogRecord),
    Metric(Metric),
}

impl ByteSizeOf for Event {
    fn allocated_bytes(&self) -> usize {
        match self {
            Event::Log(log) => log.allocated_bytes(),
            Event::Metric(metric) => metric.allocated_bytes(),
        }
    }
}

impl Finalizable for Event {
    fn take_finalizers(&mut self) -> EventFinalizers {
        match self {
            Event::Log(log) => log.take_finalizers(),
            Event::Metric(metric) => metric.take_finalizers(),
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

    #[inline]
    pub fn new_empty_log() -> Self {
        Event::Log(LogRecord::default())
    }

    pub fn add_batch_notifier(&mut self, batch: Arc<BatchNotifier>) {
        let finalizer = EventFinalizer::new(batch);
        match self {
            Self::Log(log) => log.add_finalizer(finalizer),
            Self::Metric(metric) => metric.add_finalizer(finalizer),
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
        }
    }
}

impl From<Metric> for Event {
    fn from(m: Metric) -> Self {
        Self::Metric(m)
    }
}

impl EncodeBytes<Event> for Event {
    type Error = EncodeError;

    fn encode<B>(self, _buffer: &mut B) -> Result<(), Self::Error>
    where
        B: BufMut,
        Self: Sized,
    {
        todo!()
    }
}

impl DecodeBytes<Event> for Event {
    type Error = DecodeError;

    fn decode<B>(_buffer: B) -> Result<Event, Self::Error>
    where
        Event: Sized,
        B: Buf,
    {
        todo!()
    }
}

impl From<LogRecord> for Event {
    fn from(r: LogRecord) -> Self {
        Self::Log(r)
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

impl From<&str> for Event {
    fn from(s: &str) -> Self {
        let log = LogRecord::from(s);
        Self::Log(log)
    }
}

/// A wrapper for references to inner event types, where reconstituting
/// a full `Event` from a `LogEvent` or `Metric` might be inconvenient.
#[derive(Clone, Copy, Debug)]
pub enum EventRef<'a> {
    Log(&'a LogRecord),
    Metric(&'a Metric),
}

impl<'a> From<&'a Event> for EventRef<'a> {
    fn from(event: &'a Event) -> Self {
        match event {
            Event::Log(log) => log.into(),
            Event::Metric(metric) => metric.into(),
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
