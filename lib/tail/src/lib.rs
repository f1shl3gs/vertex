mod buffer;
mod checkpoint;
mod harvester;
pub mod provider;
mod watch;

// re-export
pub use buffer::*;
pub use checkpoint::{Checkpointer, Fingerprint, Position};
pub use harvester::{Harvester, Line};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ReadFrom {
    Beginning,
    End,
    Checkpoint(Position),
}

impl Default for ReadFrom {
    fn default() -> Self {
        ReadFrom::Beginning
    }
}
