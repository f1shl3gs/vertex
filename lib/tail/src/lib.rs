mod buffer;
mod checkpoint;
mod harvester;
pub mod provider;
mod watch;

// re-export
pub use buffer::*;
pub use checkpoint::{Checkpointer, Fingerprint, Position};
pub use harvester::{Harvester, Line};

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub enum ReadFrom {
    #[default]
    Beginning,
    End,
    Checkpoint(Position),
}
