//! A collection of support structures that are used in the process of encoding
//! events into bytes.

mod config;
mod encoder;
mod format;
mod framing;
mod transformer;

use std::fmt::{Debug, Display, Formatter};

use bytes::BytesMut;
use configurable::Configurable;
use event::Event;
use serde::{Deserialize, Serialize};

pub use config::{EncodingConfig, EncodingConfigWithFraming, FramingConfig, SinkType};
pub use encoder::{Encoder, EncodingError};
pub use format::{
    json::JsonSerializer, logfmt::LogfmtSerializer, native_json::NativeJsonSerializer,
    text::TextSerializer,
};
pub use framing::{
    bytes::BytesEncoder,
    character::{CharacterDelimitedEncoder, CharacterDelimitedFramerConfig},
    newline::NewlineDelimitedEncoder,
};
pub use transformer::{TimestampFormat, Transformer};

use super::FramingError;

/// The error returned when serializing a structured event into bytes.
#[derive(Debug)]
pub enum SerializeError {
    /// Io error
    Io(std::io::Error),

    /// Format error
    Fmt(std::fmt::Error),

    /// Error when serializing event to json
    Json(serde_json::Error),

    /// Error when serializing event
    Other(Box<dyn std::error::Error + Sync + Send>),
}

impl From<std::io::Error> for SerializeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<std::fmt::Error> for SerializeError {
    fn from(err: std::fmt::Error) -> Self {
        Self::Fmt(err)
    }
}

impl From<serde_json::Error> for SerializeError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl Display for SerializeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializeError::Io(err) => write!(f, "IO error, {}", err),
            SerializeError::Fmt(err) => write!(f, "format error, {}", err),
            SerializeError::Json(err) => write!(f, "serialize json failed, {}", err),
            SerializeError::Other(msg) => write!(f, "unknown error, {}", msg),
        }
    }
}

/// Configuration for building a `Serializer`
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializerConfig {
    /// Configures the `JsonSerializer`
    Json,
    /// Configures the `LogfmtSerializer`
    Logfmt,
    /// Configures the `NativeJsonSerializer`
    NativeJson,
    /// Configures the `TextSerializer`
    Text,
}

impl SerializerConfig {
    /// Build the `Serializer` with this configuration.
    pub fn build(&self) -> Serializer {
        match self {
            SerializerConfig::Json => Serializer::Json(JsonSerializer::new()),
            SerializerConfig::Logfmt => Serializer::Logfmt(LogfmtSerializer::new()),
            SerializerConfig::NativeJson => Serializer::Native(NativeJsonSerializer::new()),
            SerializerConfig::Text => Serializer::Text(TextSerializer::new()),
        }
    }
}

/// Serialize structured events as bytes.
#[derive(Clone, Debug)]
pub enum Serializer {
    /// Uses a `JsonSerializer` for serialization.
    Json(JsonSerializer),
    /// Uses a `LogfmtSerializer` for serialization.
    Logfmt(LogfmtSerializer),
    /// Uses a `NativeJsonSerializer` for serialization.
    Native(NativeJsonSerializer),
    /// Uses a `TextSerializer` for serialization.
    Text(TextSerializer),
}

impl From<TextSerializer> for Serializer {
    fn from(s: TextSerializer) -> Self {
        Self::Text(s)
    }
}

impl From<JsonSerializer> for Serializer {
    fn from(s: JsonSerializer) -> Self {
        Self::Json(s)
    }
}

impl tokio_util::codec::Encoder<Event> for Serializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, buf: &mut BytesMut) -> Result<(), Self::Error> {
        match self {
            Serializer::Json(s) => s.encode(event, buf),
            Serializer::Logfmt(s) => s.encode(event, buf),
            Serializer::Native(s) => s.encode(event, buf),
            Serializer::Text(s) => s.encode(event, buf),
        }
    }
}

/// Produce a byte stream from byte frames.
#[derive(Clone, Debug)]
pub enum Framer {
    /// Uses a `BytesEncoder` for framing.
    Bytes(BytesEncoder),
    /// Uses a `CharacterDelimitedEncoder` for framing.
    CharacterDelimited(CharacterDelimitedEncoder),
    /// Uses a `NewlineDelimitedEncoder` for framing.
    NewlineDelimited(NewlineDelimitedEncoder),
}

impl From<NewlineDelimitedEncoder> for Framer {
    fn from(f: NewlineDelimitedEncoder) -> Self {
        Self::NewlineDelimited(f)
    }
}

impl From<CharacterDelimitedEncoder> for Framer {
    fn from(f: CharacterDelimitedEncoder) -> Self {
        Self::CharacterDelimited(f)
    }
}

impl tokio_util::codec::Encoder<()> for Framer {
    type Error = FramingError;

    fn encode(&mut self, _item: (), buf: &mut BytesMut) -> Result<(), Self::Error> {
        match self {
            Framer::Bytes(f) => f.encode((), buf),
            Framer::CharacterDelimited(f) => f.encode((), buf),
            Framer::NewlineDelimited(f) => f.encode((), buf),
        }
    }
}
