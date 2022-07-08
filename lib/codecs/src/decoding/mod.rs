//! A collection of support structures that are used in the process of decoding
//! bytes into events.

use serde::{Deserialize, Serialize};

/// Configuration for building a `Deserializer`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum DeserializerConfig {
    /// Configures the `BytesDeserializer`
    Bytes,
    /// Configures the `JsonDeserializer`
    Json,

    #[cfg(feature = "syslog")]
    /// Configures the `SyslogDeserializer`
    Syslog,
}
