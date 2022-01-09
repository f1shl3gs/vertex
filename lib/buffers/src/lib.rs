mod acker;
mod disk;
mod usage;
mod topology;

// re-export
pub use acker::{Ackable, Acker};

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WhenFull {
    Block,
    DropNewest,
}

impl Default for WhenFull {
    fn default() -> Self {
        Self::Block
    }
}
