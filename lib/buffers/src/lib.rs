mod acker;
mod bytes;
mod disk;
mod usage;

// re-export
pub use acker::{Acker, Ackable};
pub use crate::bytes::{DecodeBytes, EncodeBytes};

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

