use configurable::Configurable;
use serde::{Deserialize, Serialize};

#[cfg(feature = "syslog")]
use super::format::SyslogDeserializerConfig;
use super::format::{
    BytesDeserializer, JsonDeserializerConfig, LogfmtDeserializer, VtlDeserializerConfig,
};
use super::framing::{
    BytesDecoder, CharacterDelimitedDecoderConfig, NewlineDelimitedDecoderConfig,
    OctetCountingDecoderConfig,
};
use super::{Decoder, Deserializer, Framer};
use crate::error::BuildError;

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
            FramingConfig::Bytes => Framer::Bytes(BytesDecoder::new()),
            FramingConfig::CharacterDelimited(config) => Framer::CharacterDelimited(config.build()),
            FramingConfig::NewlineDelimited(config) => Framer::NewlineDelimited(config.build()),
            FramingConfig::OctetCounting(config) => Framer::OctetCounting(config.build()),
        }
    }
}

/// Configuration for building a `Deserializer`.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize, Default)]
#[serde(tag = "codec", rename_all = "lowercase")]
pub enum DeserializerConfig {
    /// Configures the `JsonDeserializer`
    Json(JsonDeserializerConfig),

    /// Configures the `BytesDeserializer`
    #[default]
    Bytes,

    /// Configures the `LogfmtDeserializer`
    Logfmt,

    #[cfg(feature = "syslog")]
    /// Configures the `SyslogDeserializer`
    Syslog(SyslogDeserializerConfig),

    /// Configures the `VtlDeserializerConfig`
    VTL(VtlDeserializerConfig),
}

impl DeserializerConfig {
    /// Build `Deserializer` with this configuration.
    pub fn build(&self) -> Result<Deserializer, BuildError> {
        let deserializer = match self {
            DeserializerConfig::Bytes => Deserializer::Bytes(BytesDeserializer),
            DeserializerConfig::Json(config) => Deserializer::Json(config.build()),
            DeserializerConfig::Logfmt => Deserializer::Logfmt(LogfmtDeserializer),
            #[cfg(feature = "syslog")]
            DeserializerConfig::Syslog(config) => Deserializer::Syslog(config.build()),
            DeserializerConfig::VTL(config) => config.build().map(Deserializer::VTL)?,
        };

        Ok(deserializer)
    }

    /// Return an appropriate default framer for the given deserializer
    pub fn default_stream_framing(&self) -> FramingConfig {
        match self {
            DeserializerConfig::Bytes
            | DeserializerConfig::Json(_)
            | DeserializerConfig::Logfmt => {
                FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default())
            }

            #[cfg(feature = "syslog")]
            DeserializerConfig::Syslog(_) => {
                FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default())
            }
            DeserializerConfig::VTL(_) => FramingConfig::Bytes,
        }
    }

    /// Returns an appropriate default framing config for the given deserializer with
    /// message based inputs.
    pub fn default_message_based_framing(&self) -> FramingConfig {
        FramingConfig::Bytes
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
    pub fn build(&self) -> Result<Decoder, BuildError> {
        let framer = self.framing.build();
        let deserializer = self.decoding.build()?;

        Ok(Decoder {
            framer,
            deserializer,
        })
    }
}
