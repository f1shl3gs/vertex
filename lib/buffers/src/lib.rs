mod acker;
mod bytes;
mod disk;
mod usage;

// re-export
pub use crate::bytes::{DecodeBytes, EncodeBytes};
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
