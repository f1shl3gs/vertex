use std::fmt::{Debug, Display, Formatter};

use configurable::schema::{generate_string_schema, SchemaGenerator, SchemaObject};
use configurable::{Configurable, GenerateError};
use serde::de::{Error, MapAccess};
use serde::ser::SerializeMap;
use serde::Deserializer;
use serde::{de, ser, Serializer};

pub const GZIP_NONE: u32 = 0;
pub const GZIP_FAST: u32 = 1;
pub const GZIP_DEFAULT: u32 = 6;
pub const GZIP_BEST: u32 = 9;

/// Compression configuration.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Compression {
    /// No compression
    #[default]
    None,

    /// Gzip compression.
    ///
    /// [gzip]: https://en.wikipedia.org/wiki/Gzip
    Gzip(flate2::Compression),

    /// Zlib compression.
    ///
    /// [zlib]: https://en.wikipedia.org/wiki/Zlib
    Zlib(flate2::Compression),
}

impl Compression {
    /// Gets whether or not this compression will actually compression the input.
    ///
    /// While it may be counterintuitive for "compression" to not compress, this is simply a
    /// consequence of designing a single type that may or may not compress so that we can avoid
    /// having to box writers at a higher-level.
    ///
    /// Some callers can benefit from knowing whether or not compression is actually taking
    /// place, as different size limitations may come into play.
    pub const fn is_compressed(&self) -> bool {
        !matches!(self, Compression::None)
    }

    pub const fn gzip_default() -> Compression {
        // flate2 doesn't have a const `default` fn, since it actually implements the `Default`
        // trait, and it doesn't have a constant for what the "default" level should be, so we
        // hard-code it here.
        Compression::Gzip(flate2::Compression::new(6))
    }

    pub const fn content_encoding(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Gzip(_) => Some("gzip"),
            Self::Zlib(_) => Some("deflate"),
        }
    }
}

impl Configurable for Compression {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        // TODO: validate!?
        Ok(generate_string_schema())
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => write!(f, "none"),
            Self::Gzip(ref level) => write!(f, "gzip({})", level.level()),
            Self::Zlib(ref level) => write!(f, "zlib({})", level.level()),
        }
    }
}

impl<'de> de::Deserialize<'de> for Compression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrMap;

        enum Algorithm {
            None,
            Gzip,
        }

        impl<'de> serde::de::Visitor<'de> for StringOrMap {
            type Value = Compression;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "none" => Ok(Compression::None),
                    "gzip" => Ok(Compression::gzip_default()),
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(v),
                        &r#""none" or "gzip""#,
                    )),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut algorithm = None;
                let mut level = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "algorithm" => {
                            if algorithm.is_some() {
                                return Err(de::Error::duplicate_field("algorithm"));
                            }

                            let value = map.next_value::<String>()?;
                            algorithm = match value.as_str() {
                                "none" => Some(Algorithm::None),
                                "gzip" => Some(Algorithm::Gzip),
                                _ => {
                                    return Err(de::Error::unknown_variant(
                                        &value,
                                        &["none", "gzip"],
                                    ))
                                }
                            };
                        }
                        "level" => {
                            if level.is_some() {
                                return Err(de::Error::duplicate_field("level"));
                            }
                            level = Some(match map.next_value::<serde_json::Value>()? {
                                serde_json::Value::Number(level) => match level.as_u64() {
                                    Some(value) if value <= 9 => {
                                        flate2::Compression::new(value as u32)
                                    }
                                    Some(_) | None => {
                                        return Err(de::Error::invalid_value(
                                            de::Unexpected::Other(&level.to_string()),
                                            &"0, 1, 2, 3, 4, 5, 6, 7, 8 or 9",
                                        ));
                                    }
                                },
                                serde_json::Value::String(level) => match level.as_str() {
                                    "none" => flate2::Compression::none(),
                                    "fast" => flate2::Compression::fast(),
                                    "default" => flate2::Compression::default(),
                                    "best" => flate2::Compression::best(),
                                    level => {
                                        return Err(de::Error::invalid_value(
                                            de::Unexpected::Str(level),
                                            &r#""none", "fast", "best" or "default""#,
                                        ));
                                    }
                                },
                                value => {
                                    return Err(de::Error::invalid_type(
                                        de::Unexpected::Other(&value.to_string()),
                                        &"integer or string",
                                    ));
                                }
                            });
                        }
                        _ => return Err(de::Error::unknown_field(&key, &["algorithm", "level"])),
                    };
                }

                match algorithm.ok_or_else(|| de::Error::missing_field("algorithm"))? {
                    Algorithm::None => match level {
                        Some(_) => Err(de::Error::unknown_field("level", &[])),
                        None => Ok(Compression::None),
                    },
                    Algorithm::Gzip => Ok(Compression::Gzip(level.unwrap_or_default())),
                }
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}

impl ser::Serialize for Compression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        let mut level = None;

        match self {
            Compression::None => map.serialize_entry("algorithm", "none")?,
            Compression::Gzip(l) => {
                map.serialize_entry("algorithm", "gzip")?;
                level = Some(*l);
            }
            Compression::Zlib(l) => {
                map.serialize_entry("algorithm", "zlib")?;
                level = Some(*l);
            }
        };

        // If there's a level present, and it's _not_ the default compression level, then
        // serialize it. We already handle deserializing as the default level when the level
        // isn't explicitly specified (but `algorithm` is) so serializing the default would
        // just clutter the serialized output.
        if let Some(level) = level {
            match level.level() {
                GZIP_NONE => map.serialize_entry("level", "none")?,
                GZIP_FAST => map.serialize_entry("level", "fast")?,
                GZIP_DEFAULT => map.serialize_entry("level", "default")?,
                GZIP_BEST => map.serialize_entry("level", "best")?,
                level => map.serialize_entry("level", &level)?,
            };
        }

        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialization_yaml() {
        let tests = [
            ("none", Compression::None),
            ("algorithm: none", Compression::None),
            ("algorithm: \"gzip\"", Compression::gzip_default()),
            (
                "algorithm: gzip\nlevel: fast",
                Compression::Gzip(flate2::Compression::fast()),
            ),
            (
                "algorithm: gzip\nlevel: default",
                Compression::gzip_default(),
            ),
            (
                "algorithm: gzip\nlevel: best",
                Compression::Gzip(flate2::Compression::best()),
            ),
            (
                "algorithm: gzip\nlevel: 2",
                Compression::Gzip(flate2::Compression::new(2)),
            ),
            (
                "algorithm: gzip\nlevel: 7",
                Compression::Gzip(flate2::Compression::new(7)),
            ),
        ];

        for (input, want) in tests {
            let compression: Compression = serde_yaml::from_str(input).unwrap();

            assert_eq!(compression, want, "input: {}", input);
        }
    }

    #[test]
    fn deserialization_json() {
        let fixtures_valid = [
            (r#""none""#, Compression::None),
            (r#"{"algorithm": "none"}"#, Compression::None),
            (
                r#"{"algorithm": "gzip"}"#,
                Compression::Gzip(flate2::Compression::default()),
            ),
            (
                r#"{"algorithm": "gzip", "level": "best"}"#,
                Compression::Gzip(flate2::Compression::best()),
            ),
            (
                r#"{
  "algorithm": "gzip",
  "level": 8
}"#,
                Compression::Gzip(flate2::Compression::new(8)),
            ),
        ];
        for (input, result) in fixtures_valid {
            let deserialized: Result<Compression, _> = serde_json::from_str(input);
            assert_eq!(deserialized.expect("valid source"), result);
        }

        let fixtures_invalid = [
            (
                r#"42"#,
                r#"invalid type: integer `42`, expected string or map at line 1 column 2"#,
            ),
            (
                r#""b42""#,
                r#"invalid value: string "b42", expected "none" or "gzip" at line 1 column 5"#,
            ),
            (
                r#"{"algorithm": "b42"}"#,
                r#"unknown variant `b42`, expected `none` or `gzip` at line 1 column 20"#,
            ),
            (
                r#"{"algorithm": "none", "level": "default"}"#,
                r#"unknown field `level`, there are no fields at line 1 column 41"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": -1}"#,
                r#"invalid value: -1, expected 0, 1, 2, 3, 4, 5, 6, 7, 8 or 9 at line 1 column 34"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": "good"}"#,
                r#"invalid value: string "good", expected "none", "fast", "best" or "default" at line 1 column 38"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": {}}"#,
                r#"invalid type: {}, expected integer or string at line 1 column 34"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": "default", "key": 42}"#,
                r#"unknown field `key`, expected `algorithm` or `level` at line 1 column 47"#,
            ),
        ];
        for (input, result) in fixtures_invalid {
            let deserialized: Result<Compression, _> = serde_json::from_str(input);
            let err = deserialized.expect_err("invalid source");
            assert_eq!(err.to_string().as_str(), result);
        }
    }
}
