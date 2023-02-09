use configurable::Configurable;
use event::Event;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::Decoder;
#[cfg(feature = "syslog")]
use crate::decoding::SyslogDeserializer;
use crate::decoding::{
    BytesDeserializer, BytesDeserializerConfig, CharacterDelimitedDecoder, DecodeError,
    Deserializer, Framer, JsonDeserializer, LogfmtDeserializer, NewlineDelimitedDecoder,
    OctetCountingDecoder,
};

/// Configuration for building a `Framer`.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configuration the `BytesFramer`
    Bytes,

    /// Configures the `NewlineDelimitedFramer`
    NewlineDelimited {
        /// The maximum length of the byte buffer.
        ///
        /// This length does *not* include the trailing delimiter.
        ///
        /// By default, there is no maximum length enforced. If events are malformed, this can lead to
        /// additional resource usage as events continue to be buffered in memory, and can potentially
        /// lead to memory exhaustion in extreme cases.
        ///
        /// If there is a risk of processing malformed data, such as logs with user-controlled input,
        /// consider setting the maximum length to a reasonably large value as a safety net. This will
        /// ensure that processing is not truly unbounded.
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
            FramingConfig::Bytes => Framer::Bytes(BytesDeserializerConfig::new()),
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
            FramingConfig::NewlineDelimited { max_length } => {
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
#[derive(Configurable, Clone, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeserializerConfig {
    /// Configures the `BytesDeserializer`
    #[default]
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

    /// Return an appropriate default framer for the given deserializer
    pub fn default_stream_framing(&self) -> FramingConfig {
        match self {
            DeserializerConfig::Bytes | DeserializerConfig::Json | DeserializerConfig::Logfmt => {
                FramingConfig::NewlineDelimited { max_length: None }
            }

            #[cfg(feature = "syslog")]
            DeserializerConfig::Syslog => FramingConfig::NewlineDelimited { max_length: None },
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
