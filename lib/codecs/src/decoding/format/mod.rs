mod json;
#[cfg(feature = "syslog")]
mod syslog;

use std::convert::Infallible;
use std::fmt::Debug;
use std::str::Utf8Error;

use bytes::Bytes;
use event::Event;
use smallvec::SmallVec;

#[derive(Debug)]
pub enum Error {
    Utf8(Utf8Error),
    Json(serde_json::Error),
    //
    Other(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self::Other(err)
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
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, Error>;
}
