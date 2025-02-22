use configurable::Configurable;
use serde::{Deserialize, Serialize};

use super::{
    BytesEncoder, CharacterDelimitedEncoder, Framer, LengthDelimitedEncoder,
    NewlineDelimitedEncoder, Serializer, SerializerConfig, transformer::Transformer,
};

/// Configuration for building a `Framer`.
#[derive(Configurable, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FramingConfig {
    /// Configure the `BytesEncoder`
    Bytes,

    /// Configure the `CharacterDelimitedEncoder`
    CharacterDelimited {
        /// Delimiter for `CharacterDelimited`
        delimiter: u8,
    },

    /// Event data is prefixed with its length in bytes.
    ///
    /// The prefix is a 32-bit unsigned integer, little endian.
    LengthDelimited,

    /// Configures the `NewlineDelimitedEncoder`
    NewlineDelimited,
}

impl FramingConfig {
    /// Build the `Framer` from this configuration
    pub fn build(&self) -> Framer {
        match self {
            FramingConfig::Bytes => Framer::Bytes(BytesEncoder::new()),
            FramingConfig::CharacterDelimited { delimiter } => {
                Framer::CharacterDelimited(CharacterDelimitedEncoder::new(*delimiter))
            }
            FramingConfig::LengthDelimited => {
                Framer::LengthDelimited(LengthDelimitedEncoder::default())
            }
            FramingConfig::NewlineDelimited => {
                Framer::NewlineDelimited(NewlineDelimitedEncoder::new())
            }
        }
    }
}

/// Encoding configuration
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct EncodingConfig {
    /// The encoding codec used to serialize the events before outputting.
    #[serde(flatten)]
    codec: SerializerConfig,

    #[serde(flatten)]
    transformer: Transformer,
}

impl EncodingConfig {
    /// Creates a new `EncodingConfig` with the provided `SerializerConfig` and `Transformer`.
    pub fn new(codec: SerializerConfig, transformer: Transformer) -> Self {
        Self { codec, transformer }
    }

    /// Get the encoding configuration.
    pub fn config(&self) -> &SerializerConfig {
        &self.codec
    }

    /// Build a `Transformer` that applies the encoding rules to an event before serialization.
    pub fn transformer(&self) -> Transformer {
        self.transformer.clone()
    }

    /// Build a `Serializer` with this configuration.
    pub fn build(&self) -> Serializer {
        self.codec.build()
    }
}

impl<T> From<T> for EncodingConfig
where
    T: Into<SerializerConfig>,
{
    fn from(s: T) -> Self {
        Self {
            codec: s.into(),
            transformer: Default::default(),
        }
    }
}

/// The way a sink processes outgoing events.
pub enum SinkType {
    /// Events are sent in a continuous stream.
    StreamBased,

    /// Events are sent in a batch as a message.
    MessageBased,
}

/// Encoding configuration with Framing
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncodingConfigWithFraming {
    encoding: EncodingConfig,

    framing: Option<FramingConfig>,
}

impl EncodingConfigWithFraming {
    /// Creates a new `EncodingConfigWithFraming` with the provided `FramingConfig`,
    /// `SerializerConfig` and `Transformer`
    pub const fn new(
        framing: Option<FramingConfig>,
        encoding: SerializerConfig,
        transformer: Transformer,
    ) -> Self {
        Self {
            framing,
            encoding: EncodingConfig {
                codec: encoding,
                transformer,
            },
        }
    }

    /// Build a `Transformer` that applies the encoding rules to an event before serialization
    pub fn transformer(&self) -> Transformer {
        self.encoding.transformer.clone()
    }

    /// Get the encoding configuration.
    pub const fn config(&self) -> (&Option<FramingConfig>, &SerializerConfig) {
        (&self.framing, &self.encoding.codec)
    }

    /// Build the `Framer` and `Serializer` for this config.
    pub fn build(&self, sink_type: SinkType) -> (Framer, Serializer) {
        let framer = self.framing.as_ref().map(|framing| framing.build());
        let serializer = self.encoding.build();

        let framer = match (framer, &serializer) {
            (Some(framer), _) => framer,
            (None, Serializer::Json(_)) => match sink_type {
                SinkType::StreamBased => NewlineDelimitedEncoder::new().into(),
                SinkType::MessageBased => CharacterDelimitedEncoder::new(b',').into(),
            },
            (None, Serializer::Logfmt(_) | Serializer::Native(_) | Serializer::Text(_)) => {
                NewlineDelimitedEncoder::new().into()
            }
        };

        (framer, serializer)
    }
}

impl<F, S> From<(Option<F>, S)> for EncodingConfigWithFraming
where
    F: Into<FramingConfig>,
    S: Into<SerializerConfig>,
{
    fn from((framing, encoding): (Option<F>, S)) -> Self {
        Self {
            framing: framing.map(Into::into),
            encoding: encoding.into().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use event::log::parse_value_path;

    use super::*;
    use crate::encoding::TimestampFormat;
    use crate::encoding::format::json::JsonSerializerConfig;

    #[test]
    fn json() {
        for (input, want) in [
            (
                r#"codec: json"#,
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
            ),
            (
                r#"
                codec: json
                pretty: false
                "#,
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
            ),
            (
                r#"
                codec: json
                pretty: true
                "#,
                SerializerConfig::Json(JsonSerializerConfig { pretty: true }),
            ),
        ] {
            let got = serde_yaml::from_str::<SerializerConfig>(input).unwrap();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn deserialize() {
        let tests = [
            (
                r##"
codec: json
only_fields:
  - a.b[0]
except_fields:
  - ignore_me
timestamp_format: unix
"##,
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
                Transformer::new(
                    Some(vec![parse_value_path("a.b[0]").unwrap()]),
                    Some(vec![parse_value_path("ignore_me").unwrap()]),
                    Some(TimestampFormat::Unix),
                )
                .unwrap(),
            ),
            (
                r##"
codec: logfmt
only_fields:
  - a.b[0]
  - b.a
except_fields:
  - ignore_me
timestamp_format: unix
"##,
                SerializerConfig::Logfmt,
                Transformer::new(
                    Some(vec![
                        parse_value_path("a.b[0]").unwrap(),
                        parse_value_path("b.a").unwrap(),
                    ]),
                    Some(vec![parse_value_path("ignore_me").unwrap()]),
                    Some(TimestampFormat::Unix),
                )
                .unwrap(),
            ),
        ];

        #[allow(unused_variables)]
        for (input, config, want) in tests {
            let encoding = serde_yaml::from_str::<EncodingConfig>(input).unwrap();
            let serializer = encoding.config();

            assert!(matches!(serializer, config));
            let got = encoding.transformer();
            assert_eq!(got, want)
        }
    }

    #[test]
    fn deserialize_config_with_framing() {
        let tests = [
            (
                r#"
framing: newline_delimited
encoding:
    codec: json
    only_fields: [ "a.b.c" ]
"#,
                Some(FramingConfig::NewlineDelimited),
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
                Transformer::new(Some(vec![parse_value_path("a.b.c").unwrap()]), None, None)
                    .unwrap(),
            ),
            (
                r#"
encoding:
    codec: json
    only_fields: [ "a.b.c" ]
"#,
                None,
                SerializerConfig::Json(JsonSerializerConfig { pretty: false }),
                Transformer::new(Some(vec![parse_value_path("a.b.c").unwrap()]), None, None)
                    .unwrap(),
            ),
        ];

        #[allow(unused_variables)]
        for (input, framing, config, transformer) in tests {
            let encoding = serde_yaml::from_str::<EncodingConfigWithFraming>(input).unwrap();
            let (got_framing, serializer) = encoding.config();

            assert!(matches!(got_framing, framing));
            assert!(matches!(serializer, config));
            assert_eq!(encoding.transformer(), transformer)
        }
    }
}
