mod log;
mod metric;
mod trace;
mod value;

use std::collections::BTreeMap;
pub use metric::*;
pub use log::LogRecord;
use buffers::{EncodeBytes, DecodeBytes};
use bytes::{BufMut, Buf};
use prost::{DecodeError, EncodeError};
pub use crate::event::value::Value;

#[macro_export]
macro_rules! tags {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}


#[macro_export]
macro_rules! fields {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, crate::event::Value> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}


#[macro_export]
macro_rules! gauge_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge($value)
        }
    };
    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge($value)
        }
    };
}

#[macro_export]
macro_rules! sum_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Sum($value.into())
        }
    };

    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: 0,
            value: MetricValue::Sum($value)
        }
    };
}


#[derive(PartialEq, PartialOrd, Debug, Clone)]
pub enum Event {
    Log(LogRecord),
    Metric(Metric),
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
}

impl From<Metric> for Event {
    fn from(m: Metric) -> Self {
        Self::Metric(m)
    }
}

impl EncodeBytes<Event> for Event {
    type Error = EncodeError;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::Error> where B: BufMut, Self: Sized {
        todo!()
        // proto::EventWrapper::from(self).encode(buffer)
    }
}

impl DecodeBytes<Event> for Event {
    type Error = DecodeError;

    fn decode<B>(buffer: B) -> Result<Event, Self::Error> where Event: Sized, B: Buf {
        todo!()
        // proto::EventWrapper::decode(buffer).map(|wrp| wrp.into())
    }
}

impl From<LogRecord> for Event {
    fn from(r: LogRecord) -> Self {
        Self::Log(r)
    }
}

impl From<BTreeMap<String, Value>> for Event {
    fn from(_: BTreeMap<String, Value>) -> Self {
        todo!()
    }
}