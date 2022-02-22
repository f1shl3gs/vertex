pub mod bytes;
pub mod json;
pub mod syslog;

use std::fmt::Debug;

use ::bytes::Bytes;
use dyn_clone::DynClone;
use event::Event;
use smallvec::SmallVec;

/// Parse structured events from bytes
pub trait Deserializer: DynClone + Debug + Send + Sync {
    /// Returns a `SmallVec` rather than an `Event` directly, since one byte
    /// frame can potentially hold multiple events, e.g. when parsing a JSON
    /// array. However, we optimize the most common case of emitting one event
    /// by not requiring heap allocations for it.
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>>;
}

dyn_clone::clone_trait_object!(Deserializer);

/// A `Box` containing a `Deserializer`
pub type BoxedDeserializer = Box<dyn Deserializer>;
