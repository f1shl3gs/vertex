//! A collection of support structures that are used in the process of decoding
//! bytes into events.

mod config;
mod error;
pub mod format;
mod framing;

use std::fmt::Debug;

use bytes::{Bytes, BytesMut};
use event::Event;
use format::{DeserializeError, Deserializer as _};
use smallvec::SmallVec;
use tracing::warn;

pub use config::*;
pub use error::{DecodeError, StreamDecodingError};
#[cfg(feature = "syslog")]
pub use format::SyslogDeserializer;
pub use format::{BytesDeserializer, JsonDeserializer, LogfmtDeserializer};
pub use framing::{
    BytesDeserializerConfig, CharacterDelimitedDecoder, CharacterDelimitedDecoderConfig,
    NewlineDelimitedDecoder, NewlineDelimitedDecoderConfig, OctetCountingDecoder,
};

use crate::FramingError;

/// Produce byte frames from a byte stream / byte message
#[derive(Clone, Debug)]
pub enum Framer {
    /// Uses a `BytesDecoder` for framing
    Bytes(BytesDeserializerConfig),
    /// Uses a `CharacterDelimitedDecoder` for framing.
    CharacterDelimited(CharacterDelimitedDecoder),
    /// Uses a `NewlineDelimitedDecoder` for framing.
    NewlineDelimited(NewlineDelimitedDecoder),
    /// Uses a `OctetCountingDecoder` for framing
    OctetCounting(OctetCountingDecoder),
}

impl From<OctetCountingDecoder> for Framer {
    fn from(f: OctetCountingDecoder) -> Self {
        Self::OctetCounting(f)
    }
}

impl From<BytesDeserializerConfig> for Framer {
    fn from(f: BytesDeserializerConfig) -> Self {
        Self::Bytes(f)
    }
}

impl tokio_util::codec::Decoder for Framer {
    type Item = Bytes;
    type Error = FramingError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            Framer::Bytes(f) => f.decode(src),
            Framer::CharacterDelimited(f) => f.decode(src),
            Framer::NewlineDelimited(f) => f.decode(src),
            Framer::OctetCounting(f) => f.decode(src),
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self {
            Framer::Bytes(f) => f.decode_eof(buf),
            Framer::CharacterDelimited(f) => f.decode_eof(buf),
            Framer::NewlineDelimited(f) => f.decode_eof(buf),
            Framer::OctetCounting(f) => f.decode_eof(buf),
        }
    }
}

/// Parse structured events from bytes
#[derive(Clone, Debug)]
pub enum Deserializer {
    /// Uses a `BytesDeserializer` for deserialization.
    Bytes(BytesDeserializer),
    /// Uses a `JsonDeserializer` for deserialization.
    Json(JsonDeserializer),
    /// Uses a `LogfmtDeserializer` for deserialization.
    Logfmt(LogfmtDeserializer),

    #[cfg(feature = "syslog")]
    /// Uses a `SyslogDeserializer` for deserialization.
    Syslog(SyslogDeserializer),
}

#[cfg(feature = "syslog")]
impl From<SyslogDeserializer> for Deserializer {
    fn from(d: SyslogDeserializer) -> Self {
        Self::Syslog(d)
    }
}

impl format::Deserializer for Deserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        match self {
            Deserializer::Bytes(d) => d.parse(buf),
            Deserializer::Json(d) => d.parse(buf),
            Deserializer::Logfmt(d) => d.parse(buf),
            #[cfg(feature = "syslog")]
            Deserializer::Syslog(d) => d.parse(buf),
        }
    }
}

/// A decoder that can decode structured events from a byte stream / byte
/// messages.
#[derive(Clone, Debug)]
pub struct Decoder {
    framer: Framer,
    deserializer: Deserializer,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            framer: Framer::NewlineDelimited(NewlineDelimitedDecoder::new()),
            deserializer: Deserializer::Bytes(BytesDeserializer),
        }
    }
}

impl Decoder {
    /// Create a new `Decoder` with framer and deserializer.
    pub fn new(framer: Framer, deserializer: Deserializer) -> Self {
        Self {
            framer,
            deserializer,
        }
    }

    /// Handles the framing result and parses it into a structured event, if
    /// possible.
    ///
    /// Emits logs if either framing or parsing failed.
    #[allow(clippy::type_complexity)]
    fn handle_framing_result(
        &mut self,
        frame: Result<Option<Bytes>, FramingError>,
    ) -> Result<Option<(SmallVec<[Event; 1]>, usize)>, DecodeError> {
        let frame = frame.map_err(|err| {
            warn!(
                message = "Failed framing bytes",
                ?err,
                internal_log_rate_limit = true
            );
            DecodeError::Framing(err)
        })?;

        let frame = match frame {
            Some(frame) => frame,
            _ => return Ok(None),
        };

        let byte_size = frame.len();
        // Parse structured events from the byte frame.
        self.deserializer
            .parse(frame)
            .map(|events| Some((events, byte_size)))
            .map_err(DecodeError::Deserialize)
    }
}

impl tokio_util::codec::Decoder for Decoder {
    type Item = (SmallVec<[Event; 1]>, usize);
    type Error = DecodeError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode(src);
        self.handle_framing_result(frame)
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let frame = self.framer.decode_eof(buf);
        self.handle_framing_result(frame)
    }
}
