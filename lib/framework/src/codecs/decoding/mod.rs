pub mod format;
mod framing;

use bytes::{Bytes, BytesMut};
use event::Event;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt::{Debug, Formatter};
use tokio_util::codec::LinesCodecError;

pub use format::bytes::{BytesDeserializer, BytesDeserializerConfig};
use format::json::{JsonDeserializer, JsonDeserializerConfig};
pub use format::syslog::{SyslogDeserializer, SyslogDeserializerConfig};
use format::BoxedDeserializer;
use format::Deserializer as _;
pub use framing::bytes::{BytesDecoder, BytesDecoderConfig};
pub use framing::character::{
    CharacterDelimitedDecoder, CharacterDelimitedDecoderConfig, CharacterDelimitedDecoderOptions,
};
pub use framing::newline::{
    NewlineDelimitedDecoder, NewlineDelimitedDecoderConfig, NewlineDelimitedDecoderOptions,
};
pub use framing::octet_counting::OctetCountingDecoder;
use framing::BoxedFramingError;

use super::StreamDecodingError;
use crate::codecs::decoding::framing::octet_counting::{
    OctetCountingDecoderConfig, OctetCountingDecoderOptions,
};
use crate::config::{skip_serializing_if_default, GenerateConfig};

/// An error that occurred while decoding structured events from a byte stream
/// byte message
#[derive(Debug)]
pub enum Error {
    /// The error occurred while producing byte frames from the byte
    /// stream byte messages
    Framing(BoxedFramingError),
    /// The error occurred while parsing structured events from a byte frame
    Parsing(crate::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Framing(err) => write!(f, "Framing({})", err),
            Self::Parsing(err) => write!(f, "Parsing({})", err),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Framing(Box::new(err))
    }
}

impl StreamDecodingError for Error {
    fn can_continue(&self) -> bool {
        match self {
            Self::Framing(err) => err.can_continue(),
            Self::Parsing(_) => true,
        }
    }
}

impl StreamDecodingError for std::io::Error {
    fn can_continue(&self) -> bool {
        false
    }
}

impl StreamDecodingError for LinesCodecError {
    fn can_continue(&self) -> bool {
        match self {
            LinesCodecError::MaxLineLengthExceeded => true,
            LinesCodecError::Io(err) => err.can_continue(),
        }
    }
}

/// Configuration for building a `Framer`
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configures the `BytesDecoder`
    Bytes,
    /// Configures the `CharacterDelimitedDecoder`.
    CharacterDelimited {
        /// Options for the character delimited decoder.
        character_delimited: CharacterDelimitedDecoderOptions,
    },
    /// Configures the `NewlineDelimitedDecoder`
    NewlineDelimited {
        #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
        newline_delimited: NewlineDelimitedDecoderOptions,
    },
    /// Configures the `OctetCountingDecoder`
    OctetCounting {
        #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
        octet_counting: OctetCountingDecoderOptions,
    },
}

impl From<BytesDecoderConfig> for FramingConfig {
    fn from(_: BytesDecoderConfig) -> Self {
        Self::Bytes
    }
}

impl From<NewlineDelimitedDecoderConfig> for FramingConfig {
    fn from(config: NewlineDelimitedDecoderConfig) -> Self {
        Self::NewlineDelimited {
            newline_delimited: config.newline_delimited,
        }
    }
}

impl FramingConfig {
    fn build(self) -> Framer {
        match self {
            FramingConfig::Bytes => Framer::Bytes(BytesDecoderConfig.build()),
            FramingConfig::CharacterDelimited {
                character_delimited,
            } => Framer::CharacterDelimited(
                CharacterDelimitedDecoderConfig {
                    character_delimited,
                }
                .build(),
            ),
            FramingConfig::NewlineDelimited { newline_delimited } => Framer::NewlineDelimited(
                NewlineDelimitedDecoderConfig { newline_delimited }.build(),
            ),
            FramingConfig::OctetCounting { octet_counting } => {
                Framer::OctetCounting(OctetCountingDecoderConfig { octet_counting }.build())
            }
        }
    }
}

impl GenerateConfig for FramingConfig {
    fn generate_config() -> String {
        r#"
# The framing method
#
# Available options:
#   bytes:               Byte frames are passed through as-is according to the underlying
#                        I/O boundaries
#   character_delimited: Bytes frames which are delimited by a chosen charactor
#   newline_delimited:   Bytes frames which are delimited by a newilne charactor
#
method: newline_delimited

# The maximum frame lenghth limit. Any frames loger than `max_length` bytes
# will be discarded entirely.
#
max_length: 16KiB
        "#
        .into()
    }
}

/// Produce byte frames from a byte stream / byte message
#[derive(Clone, Debug)]
pub enum Framer {
    /// Uses a `BytesDecoder` for framing
    Bytes(BytesDecoder),
    /// Uses a `CharacterDelimitedDecoder` for framing
    CharacterDelimited(CharacterDelimitedDecoder),
    /// Uses a `NewlineDelimitedDecoder` for framing
    NewlineDelimited(NewlineDelimitedDecoder),
    /// Uses a `` for framing
    OctetCounting(OctetCountingDecoder),
}

impl From<BytesDecoder> for Framer {
    fn from(decoder: BytesDecoder) -> Self {
        Self::Bytes(decoder)
    }
}

impl From<OctetCountingDecoder> for Framer {
    fn from(decoder: OctetCountingDecoder) -> Self {
        Self::OctetCounting(decoder)
    }
}

impl tokio_util::codec::Decoder for Framer {
    type Item = Bytes;
    type Error = BoxedFramingError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            Framer::Bytes(framer) => framer.decode(src),
            Framer::CharacterDelimited(framer) => framer.decode(src),
            Framer::NewlineDelimited(framer) => framer.decode(src),
            Framer::OctetCounting(framer) => framer.decode(src),
        }
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            Framer::Bytes(framer) => framer.decode_eof(src),
            Framer::CharacterDelimited(framer) => framer.decode_eof(src),
            Framer::NewlineDelimited(framer) => framer.decode_eof(src),
            Framer::OctetCounting(framer) => framer.decode_eof(src),
        }
    }
}

/// Configuration for building a `Deserializer`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum DeserializerConfig {
    Bytes,
    Json,
    Syslog,
}

impl From<BytesDeserializerConfig> for DeserializerConfig {
    fn from(_: BytesDeserializerConfig) -> Self {
        Self::Bytes
    }
}

impl DeserializerConfig {
    fn build(&self) -> Deserializer {
        match self {
            DeserializerConfig::Bytes => Deserializer::Bytes(BytesDeserializerConfig.build()),
            DeserializerConfig::Json => Deserializer::Json(JsonDeserializerConfig.build()),
            DeserializerConfig::Syslog => Deserializer::Syslog(SyslogDeserializerConfig.build()),
        }
    }
}

/// Parse structured events from bytes.
#[derive(Debug, Clone)]
pub enum Deserializer {
    /// Uses a `BytesDeserializer` for deserialization.
    Bytes(BytesDeserializer),
    /// Uses a `JsonDeserializer` for deserialization.
    Json(JsonDeserializer),
    /// Uses a `SyslogDeserializer` for deserialization.
    Syslog(SyslogDeserializer),
    /// Uses an opaque `Deserializer` implementation for deserialization.
    Boxed(BoxedDeserializer),
}

impl From<SyslogDeserializer> for Deserializer {
    fn from(deserializer: SyslogDeserializer) -> Self {
        Self::Syslog(deserializer)
    }
}

impl format::Deserializer for Deserializer {
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>> {
        match self {
            Deserializer::Bytes(deserializer) => deserializer.parse(bytes),
            Deserializer::Json(deserializer) => deserializer.parse(bytes),
            Deserializer::Syslog(deserializer) => deserializer.parse(bytes),
            Deserializer::Boxed(deserializer) => deserializer.parse(bytes),
        }
    }
}

/// A decoder that can decode structured events from a byte stream / byte
/// message
#[derive(Debug, Clone)]
pub struct Decoder {
    framer: Framer,
    deserializer: Deserializer,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            framer: Framer::NewlineDelimited(NewlineDelimitedDecoder::new()),
            deserializer: Deserializer::Bytes(BytesDeserializer::new()),
        }
    }
}

impl Decoder {
    /// Creates a new `Decoder` with the specified `Framer` to produce byte
    /// frames from the byte stream / byte messages and `Deserializer` to parse
    /// structured events from a byte frame
    pub fn new(framer: Framer, deserializer: Deserializer) -> Self {
        Self {
            framer,
            deserializer,
        }
    }

    /// Handles the framing result and parses it into a structured event, if
    /// possible
    ///
    /// Emits logs if either framing or parsing failed
    fn handle_framing_result(
        &mut self,
        frame: Result<Option<Bytes>, BoxedFramingError>,
    ) -> Result<Option<(SmallVec<[Event; 1]>, usize)>, Error> {
        let frame = frame.map_err(|err| {
            warn!(
                message = "Failed framing bytes",
                %err,
                internal_log_rate_secs = 10
            );
            Error::Framing(err)
        })?;

        let frame = match frame {
            Some(frame) => frame,
            _ => return Ok(None),
        };

        let byte_size = frame.len();

        // Parse structured events from the byte frame
        self.deserializer
            .parse(frame)
            .map(|event| Some((event, byte_size)))
            .map_err(|err| {
                warn!(
                    message = "Failed deserializing frame",
                    %err,
                    internal_log_rate_secs = 10
                );

                Error::Parsing(err)
            })
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = (SmallVec<[Event; 1]>, usize);
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode(src);
        self.handle_framing_result(frame)
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode_eof(buf);
        self.handle_framing_result(frame)
    }
}

/// Config used to build a `Decoder`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecodingConfig {
    /// The framing config
    framing: FramingConfig,
    /// The decoding config
    decoding: DeserializerConfig,
}

impl DecodingConfig {
    /// Creates a new `DecodingConfig` with the provided `FramingConfig` and `DeserializerConfig`
    pub fn new(framing: impl Into<FramingConfig>, decoding: impl Into<DeserializerConfig>) -> Self {
        Self {
            framing: framing.into(),
            decoding: decoding.into(),
        }
    }

    /// Builds a `Decoder` from the provided configuration.
    pub fn build(self) -> Decoder {
        // Build the framer.
        let framer = self.framing.build();

        // Build the deserializer.
        let deserializer = self.decoding.build();

        Decoder::new(framer, deserializer)
    }
}
