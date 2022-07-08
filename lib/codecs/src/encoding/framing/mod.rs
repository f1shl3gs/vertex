mod bytes;
mod character;
mod newline;

use serde::{Deserialize, Serialize};

/// Configuration for building a `Framer`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configure the `BytesEncoder`
    Bytes,

    /// Configure the `CharacterDelimitedEncoder`
    CharacterDelimited { delimiter: u8 },

    /// Configures the `NewlineDelimitedEncoder`
    NewlineDelimited,
}
