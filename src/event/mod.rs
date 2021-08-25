mod log;
mod metric;
mod trace;
mod value;

pub use metric::*;
pub use log::LogRecord;
use crate::buffers::bytes::{EncodeBytes, DecodeBytes};
use bytes::{BufMut, Buf};
use prost::{DecodeError, EncodeError};

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
