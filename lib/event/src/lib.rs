mod log;
mod metric;
mod trace;
mod value;
mod macros;

use std::collections::BTreeMap;
pub use metric::*;
pub use log::LogRecord;
pub use value::Value;
use buffers::{EncodeBytes, DecodeBytes};
use bytes::{BufMut, Buf};
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

impl ByteSizeOf for String {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl<K, V> ByteSizeOf for BTreeMap<K, V>
    where
        K: ByteSizeOf,
        V: ByteSizeOf
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, (k, v)| acc + k.size_of() + v.size_of())
    }
}

impl<T> ByteSizeOf for Vec<T>
    where
        T: ByteSizeOf
{
    fn allocated_bytes(&self) -> usize {
        self.iter()
            .fold(0, |acc, i| acc + i.size_of())
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

impl Event {
    /// This function panics if self is anything other than an `Event::Metric`
    pub fn as_mut_metric(&mut self) -> &mut Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self)
        }
    }

    pub fn as_metric(&self) -> &Metric {
        match self {
            Event::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {:?} is not a metric", self)
        }
    }

    pub fn into_metric(self) -> Metric {
        match self {
            Event::Metric(m) => m,
            _ => panic!("Failed type coercion, {:?} is not a metric", self)
        }
    }

    pub fn as_log(&self) -> &LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self)
        }
    }

    pub fn as_mut_log(&mut self) -> &mut LogRecord {
        match self {
            Event::Log(l) => l,
            _ => panic!("Failed type coercion, {:?} is not a log", self)
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

    fn encode<B>(self, _buffer: &mut B) -> Result<(), Self::Error> where B: BufMut, Self: Sized {
        todo!()
    }
}

impl DecodeBytes<Event> for Event {
    type Error = DecodeError;

    fn decode<B>(_buffer: B) -> Result<Event, Self::Error> where Event: Sized, B: Buf {
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
        Self::Log(LogRecord {
            time_unix_nano: 0,
            tags: Default::default(),
            fields: m,
        })
    }
}

impl From<String> for Event {
    fn from(s: String) -> Self {
        let mut fields: BTreeMap<String, Value> = BTreeMap::new();
        fields.insert("message".to_string(), Value::String(s));

        Self::Log(LogRecord {
            time_unix_nano: 0,
            tags: Default::default(),
            fields,
        })
    }
}