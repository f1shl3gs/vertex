#[deny(clippy::cast_precision_loss)]
mod acker;
mod buffer_usage_data;
mod config;
mod disk;
pub mod encoding;
pub mod topology;
mod variant;

#[cfg(test)]
mod test;

// re-export
pub use acker::{Ackable, Acker};
pub use config::{memory_buffer_default_max_events, BufferBuildError, BufferConfig, BufferType};
pub use encoding::{DecodeBytes, EncodeBytes};
pub use topology::{builder, channel};

use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;
use std::fmt::Debug;

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate metrics;

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
    ByteSizeOf + EncodeBytes + DecodeBytes + Debug + Send + Sync + Unpin + Sized + 'static
{
}

// Blanket implementation for anything that is already bufferable.
impl<T> Bufferable for T where
    T: ByteSizeOf + EncodeBytes + DecodeBytes + Debug + Send + Sync + Unpin + Sized + 'static
{
}
