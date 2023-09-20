use configurable::Configurable;
use serde::{Deserialize, Serialize};

use super::Decoder;
use crate::decoding::framing::OctetCountingDecoderConfig;
#[cfg(feature = "syslog")]
use crate::decoding::SyslogDeserializer;
use crate::decoding::{
    BytesDeserializer, BytesDeserializerConfig, CharacterDelimitedDecoderConfig, Deserializer,
    Framer, JsonDeserializer, LogfmtDeserializer, NewlineDelimitedDecoderConfig,
};

/// Configuration for building a `Framer`.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configuration the `BytesFramer`
    Bytes,

    /// Configures the `NewlineDelimitedFramer`
    NewlineDelimited(NewlineDelimitedDecoderConfig),

    /// Configures the `CharacterDelimitedFramer`.
    CharacterDelimited(CharacterDelimitedDecoderConfig),

    /// Configures the `OctetCountingFramer`.
    OctetCounting(OctetCountingDecoderConfig),
}

impl FramingConfig {
    /// Build a `Framer` for this configuration.
    pub fn build(&self) -> Framer {
        match self {
            FramingConfig::Bytes => Framer::Bytes(BytesDeserializerConfig::new()),
            FramingConfig::CharacterDelimited(config) => Framer::CharacterDelimited(config.build()),
            FramingConfig::NewlineDelimited(config) => Framer::NewlineDelimited(config.build()),
            FramingConfig::OctetCounting(config) => Framer::OctetCounting(config.build()),
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
                FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default())
            }

            #[cfg(feature = "syslog")]
            DeserializerConfig::Syslog => {
                FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default())
            }
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
