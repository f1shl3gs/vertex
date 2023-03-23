//! A collection of support structures that are used in the process of decoding
//! bytes into events.

mod bytes;
mod json;
mod logfmt;
#[cfg(feature = "syslog")]
mod syslog;

use std::convert::Infallible;
use std::fmt::Debug;
use std::str::Utf8Error;

use ::bytes::Bytes;
use event::Event;
use smallvec::SmallVec;

pub use self::bytes::*;
use crate::FramingError;
pub use json::JsonDeserializer;
pub use logfmt::LogfmtDeserializer;
#[cfg(feature = "syslog")]
pub use syslog::SyslogDeserializer;

/// An error that occurred while decoding structured events from a byte stream /
/// byte messages.
#[derive(Debug)]
pub enum DeserializeError {
    /// The error occurred while converting to UTF8
    Utf8(Utf8Error),

    /// The error occurred while deserializing it from JSON
    Json(serde_json::Error),

    /// The error occurred while deserializing it from syslog RFC5424.
    Syslog(syslog::Error),

    /// The error occurred while deserializing
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<Utf8Error> for DeserializeError {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<serde_json::Error> for DeserializeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for DeserializeError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Other(err)
    }
}

impl From<String> for DeserializeError {
    fn from(s: String) -> Self {
        DeserializeError::Other(s.into())
    }
}

/// Parse structured events from bytes
pub trait Deserializer: Clone + Debug + Send + Sync {
    /// Parses structured events from bytes.
    ///
    /// It returns a `SmallVec` rather than an `Event` directly, since one byte
    /// frame can potentially hold multiple events, e.g. when parsing a JSON array.
    /// However, we optimize the most common case of emitting one event by not
    /// requiring heap allocations for it.
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError>;
}
