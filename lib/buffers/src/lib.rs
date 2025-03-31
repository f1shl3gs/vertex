#![allow(async_fn_in_trait)]

mod buffer_usage_data;
mod config;
pub mod encoding;
mod event;
pub mod topology;
mod variants;

#[cfg(test)]
mod test;

// re-export
pub use config::{BufferBuildError, BufferConfig, BufferType, memory_buffer_default_max_events};
pub use encoding::Encodable;
pub use topology::{builder, channel};

use std::fmt::Debug;

use bytesize::ByteSizeOf;
use finalize::AddBatchNotifier;
use serde::{Deserialize, Serialize};

#[macro_use]
extern crate tracing;

/// Event handling behavior when a buffer is full
#[derive(Debug, Copy, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenFull {
    /// Wait for free space in the buffer
    ///
    /// This applies backpressure up the topology. signalling that sources should slow
    /// down the acceptance/consumption of events. This means that while no data is lost,
    /// data will pile up at the edge.
    #[default]
    Block,

    /// Drops the event instead of waiting for free space in buffer
    ///
    /// The event will be intentionally dropped. This mode is typically used when performance
    /// is the highest priority, and it is preferable to temporarily lose events rather than
    /// cause a slowdown in the acceptance/consumption of events.
    DropNewest,

    /// Overflows to the next stage in the buffer topology.
    ///
    /// If the current buffer stage is full, attempt to send this event to the next buffer
    /// stage. That stage may also be configured overflow, and so on, but ultimately the
    /// last stage in a buffer topology must use one of the other handling behaviors. This
    /// means that next stage may potentially be able to buffer the event, but it may also
    /// block or drop the event.
    ///
    /// This mode can only be used when two or more buffer stages are configured.
    Overflow,
}

#[cfg(test)]
impl quickcheck::Arbitrary for WhenFull {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
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

/// Vertex's basic error type, dynamically dispatched and safe to send across
/// threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Vertex's basic result type, defined in terms of [`Error`] and generic over
/// `T`.
pub type Result<T> = std::result::Result<T, Error>;
