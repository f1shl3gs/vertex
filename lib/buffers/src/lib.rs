#[deny(clippy::cast_precision_loss)]
mod buffer_usage_data;
mod config;
pub mod encoding;
mod event;
pub mod topology;
mod variants;

#[cfg(test)]
mod test;

// re-export
pub use config::{memory_buffer_default_max_events, BufferBuildError, BufferConfig, BufferType};
pub use encoding::Encodable;
pub use topology::{builder, channel};

use std::fmt::Debug;

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use finalize::AddBatchNotifier;
#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

#[macro_use]
extern crate tracing;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenFull {
    Block,
    DropNewest,
    Overflow,
}

#[cfg(test)]
impl Arbitrary for WhenFull {
    fn arbitrary(g: &mut Gen) -> Self {
        // TODO: We explictly avoid generating "overflow" as a possible value because
        // nothing yet supports handling it, and will be defaulted to using "block"
        // if they encounter "overflow". Thus, there's no reason to emit it here... yet
        if bool::arbitrary(g) {
            WhenFull::Block
        } else {
            WhenFull::DropNewest
        }
    }
}

impl Default for WhenFull {
    fn default() -> Self {
        Self::Block
    }
}

/// An item that can be buffered.
///
/// This supertrait serves as the base trait for any item that can be pushed into a buffer.
pub trait Bufferable:
    AddBatchNotifier
    + ByteSizeOf
    + Encodable
    + EventCount
    + Debug
    + Send
    + Sync
    + Unpin
    + Sized
    + 'static
{
}

// Blanket implementation for anything that is already bufferable.
impl<T> Bufferable for T where
    T: AddBatchNotifier
        + ByteSizeOf
        + Encodable
        + EventCount
        + Debug
        + Send
        + Sync
        + Unpin
        + Sized
        + 'static
{
}

pub trait EventCount {
    fn event_count(&self) -> usize;
}

/// Vector's basic error type, dynamically dispatched and safe to send across
/// threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Vector's basic result type, defined in terms of [`Error`] and generic over
/// `T`.
pub type Result<T> = std::result::Result<T, Error>;
