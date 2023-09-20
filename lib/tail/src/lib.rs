mod buffer;
mod checkpoint;
mod harvester;
pub mod provider;
mod watch;

#[macro_use]
extern crate tracing;

// re-export
pub use buffer::*;
pub use checkpoint::{Checkpointer, Fingerprint, Position};
pub use harvester::{Harvester, Line};

#[derive(Copy, Clone, Debug, Default)]
pub enum ReadFrom {
    #[default]
    Beginning,
    End,
    Checkpoint(Position),
}
