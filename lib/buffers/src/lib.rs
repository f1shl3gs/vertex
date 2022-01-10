mod acker;
mod buffer_usage_data;
mod config;
mod disk;
mod encoding;
mod topology;
mod variant;

// re-export
pub use acker::{Ackable, Acker};
pub use config::{memory_buffer_default_max_events, BufferBuildError, BufferConfig, BufferType};
pub use encoding::{DecodeBytes, EncodeBytes};
pub use topology::{builder, channel};

use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;
use std::fmt::Debug;

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

impl Default for WhenFull {
    fn default() -> Self {
        Self::Block
    }
}

/// An item that can be buffered.
///
/// This supertrait serves as the base trait for any item that can be pushed into a buffer.
pub trait Bufferable:
    ByteSizeOf + EncodeBytes<Self> + DecodeBytes<Self> + Debug + Send + Sync + Unpin + Sized + 'static
{
}

// Blanket implementation for anything that is already bufferable.
impl<T> Bufferable for T where
    T: ByteSizeOf + EncodeBytes<T> + DecodeBytes<T> + Debug + Send + Sync + Unpin + Sized + 'static
{
}
