//! A collection of support structures that are used in the process of encoding
//! events into bytes.

mod format;
mod framing;

use serde::{Deserialize, Serialize};

/// Configuration for building a `Serializer`
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializerConfig {
    /// Configures the `JsonSerializer`
    Json,
    /// Configures the `LogfmtSerializer`
    Logfmt,
    /// Configures the `NativeJsonSerializer`,
    NativeJson,
    /// Configures the `TextSerializer`
    Text,
}

impl SerializerConfig {}
