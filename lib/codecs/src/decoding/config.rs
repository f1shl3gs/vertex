use event::Event;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::Decoder;
#[cfg(feature = "syslog")]
use crate::decoding::SyslogDeserializer;
use crate::decoding::{
    BytesDecoder, BytesDeserializer, CharacterDelimitedDecoder, DecodeError, Deserializer, Framer,
    JsonDeserializer, LogfmtDeserializer, NewlineDelimitedDecoder, OctetCountingDecoder,
};

/// Configuration for building a `Framer`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configuration the `BytesFramer`
    Bytes,

    /// Configures the `NewlineDelimitedFramer`
    NewLineDelimited {
        /// The maximum length of the byte buffer.
        ///
        /// This length does *not* include the trailing delimiter.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },

    /// Configures the `CharacterDelimitedFramer`.
    CharacterDelimited {
        /// The character that delimits byte sequences.
        ///
        /// This length does *not* include the trailing delimiter.
        delimiter: u8,

        /// The maximum length of the byte buffer.
        ///
        /// This length does *not* include the trailing delimiter.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },

    /// Configures the `OctetCountingFramer`.
    OctetCounting {
        /// The maximum length of the byte buffer.
        #[serde(skip_serializing_if = "Option::is_none")]
        max_length: Option<usize>,
    },
}

impl FramingConfig {
    /// Build a `Framer` for this configuration.
    pub fn build(&self) -> Framer {
        match self {
            FramingConfig::Bytes => Framer::Bytes(BytesDecoder::new()),
            FramingConfig::CharacterDelimited {
                delimiter,
                max_length,
            } => {
                let framer = match max_length {
                    Some(max_length) => {
                        CharacterDelimitedDecoder::new_with_max_length(*delimiter, *max_length)
                    }
                    None => CharacterDelimitedDecoder::new(*delimiter),
                };

                Framer::CharacterDelimited(framer)
            }
            FramingConfig::NewLineDelimited { max_length } => {
                let framer = match max_length {
                    Some(max_length) => NewlineDelimitedDecoder::new_with_max_length(*max_length),
                    None => NewlineDelimitedDecoder::new(),
                };

                Framer::NewlineDelimited(framer)
            }
            FramingConfig::OctetCounting { max_length } => {
                let framer = match max_length {
                    Some(max_length) => OctetCountingDecoder::new_with_max_length(*max_length),
                    None => OctetCountingDecoder::new(),
                };

                Framer::OctetCounting(framer)
            }
        }
    }
}

/// Configuration for building a `Deserializer`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "codec", rename_all = "snake_case")]
pub enum DeserializerConfig {
    /// Configures the `BytesDeserializer`
    Bytes,
    /// Configures the `JsonDeserializer`
    Json,
    /// Configures the `LogfmtDeserializer`
    Logfmt,

    #[cfg(feature = "syslog")]
    /// Configures the `SyslogDeserializer`
    Syslog,
}

impl DeserializerConfig {
    /// Build `Deserializer` with this configuration.
    pub fn build(&self) -> Deserializer {
        match self {
            DeserializerConfig::Bytes => Deserializer::Bytes(BytesDeserializer),
            DeserializerConfig::Json => Deserializer::Json(JsonDeserializer),
            DeserializerConfig::Logfmt => Deserializer::Logfmt(LogfmtDeserializer),
            #[cfg(feature = "syslog")]
            DeserializerConfig::Syslog => Deserializer::Syslog(SyslogDeserializer),
        }
    }
}

/// Config used to build a `Decoder`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DecodingConfig {
    /// The framing config.
    framing: FramingConfig,
    /// The decoding config
    decoding: DeserializerConfig,
}

impl DecodingConfig {
    /// Creates a new `DecodingConfig` with the provided `FramingConfig` and
    /// `DeserializerConfig`.
    pub fn new(framing: FramingConfig, decoding: DeserializerConfig) -> Self {
        Self { framing, decoding }
    }

    /// Build `Decoder` with this configuration.
    pub fn build(&self) -> Decoder {
        Decoder {
            framer: self.framing.build(),
            deserializer: self.decoding.build(),
        }
    }
}
