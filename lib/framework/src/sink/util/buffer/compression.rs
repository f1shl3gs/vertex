use std::fmt::{Debug, Display, Formatter};

use configurable::schema::{SchemaGenerator, SchemaObject};
use configurable::Configurable;
use serde::de::{Error, MapAccess};
use serde::ser::SerializeMap;
use serde::{de, ser, Serializer};
use serde::{Deserializer, Serialize};

use crate::sink::util::zstd::ZstdCompressionLevel;

/// Compression Level
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompressionLevel {
    None,
    #[default]
    Default,
    Best,
    Fast,
    Value(u32),
}

impl<'de> de::Deserialize<'de> for CompressionLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NumberOrString;

        impl de::Visitor<'_> for NumberOrString {
            type Value = CompressionLevel;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("unsigned number or string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let level = match v {
                    "none" => CompressionLevel::None,
                    "fast" => CompressionLevel::Fast,
                    "default" => CompressionLevel::Default,
                    "best" => CompressionLevel::Best,
                    level => {
                        return Err(de::Error::invalid_value(
                            de::Unexpected::Str(level),
                            &r#""none", "fast", "best" or "default""#,
                        ))
                    }
                };

                Ok(level)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                u32::try_from(v)
                    .map(CompressionLevel::Value)
                    .map_err(|err| {
                        Error::custom(format!(
                            "unsigned integer could not be converted to u32: {err}"
                        ))
                    })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                u32::try_from(v)
                    .map(CompressionLevel::Value)
                    .map_err(|err| {
                        Error::custom(format!("integer could not be converted to u32: {err}"))
                    })
            }
        }

        deserializer.deserialize_any(NumberOrString)
    }
}

impl Serialize for CompressionLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match *self {
            CompressionLevel::None => "none",
            CompressionLevel::Default => "default",
            CompressionLevel::Best => "best",
            CompressionLevel::Fast => "fast",
            CompressionLevel::Value(value) => return serializer.serialize_u32(value),
        };

        serializer.serialize_str(s)
    }
}

impl CompressionLevel {
    pub fn as_flate2(&self) -> flate2::Compression {
        match self {
            CompressionLevel::None => flate2::Compression::none(),
            CompressionLevel::Default => flate2::Compression::default(),
            CompressionLevel::Best => flate2::Compression::best(),
            CompressionLevel::Fast => flate2::Compression::fast(),
            CompressionLevel::Value(level) => flate2::Compression::new(*level),
        }
    }
}

/// Compression configuration.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub enum Compression {
    /// No compression
    #[default]
    None,

    /// Gzip compression.
    ///
    /// [gzip]: https://en.wikipedia.org/wiki/Gzip
    Gzip(CompressionLevel),

    /// Zlib compression.
    ///
    /// [zlib]: https://en.wikipedia.org/wiki/Zlib
    Zlib(CompressionLevel),

    /// Zstandard compression.
    ///
    /// [zstd]: https://facebook.github.io/zstd/
    Zstd(CompressionLevel),

    /// Snappy compression.
    ///
    /// [snappy]: https://github.com/google/snappy/blob/main/docs/README.md
    Snappy,
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
        Compression::Gzip(CompressionLevel::Default)
    }

    pub const fn zlib_default() -> Compression {
        Compression::Zlib(CompressionLevel::Default)
    }

    pub fn compression_level(&self) -> CompressionLevel {
        match self {
            Compression::None | Compression::Snappy => CompressionLevel::None,
            Compression::Gzip(level) | Compression::Zlib(level) | Compression::Zstd(level) => {
                *level
            }
        }
    }

    pub const fn max_compression_level_value(&self) -> u32 {
        match self {
            Compression::None | Compression::Snappy => 0,
            Compression::Gzip(_) | Compression::Zlib(_) => 9,
            Compression::Zstd(_) => 21,
        }
    }

    pub const fn content_encoding(self) -> Option<&'static str> {
        match self {
            Compression::None => None,
            Compression::Gzip(_) => Some("gzip"),
            Compression::Zlib(_) => Some("deflate"),
            Compression::Zstd(_) => Some("zstd"),
            Compression::Snappy => Some("snappy"),
        }
    }
}

impl Configurable for Compression {
    fn generate_schema(gen: &mut SchemaGenerator) -> SchemaObject {
        String::generate_schema(gen)
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Compression::None => f.write_str("none"),
            Compression::Gzip(ref level) => write!(f, "gzip({})", level.as_flate2().level()),
            Compression::Zlib(ref level) => write!(f, "zlib({})", level.as_flate2().level()),
            Compression::Zstd(ref level) => {
                write!(f, "zstd({})", ZstdCompressionLevel::from(*level))
            }
            Compression::Snappy => f.write_str("snappy"),
        }
    }
}

impl<'de> de::Deserialize<'de> for Compression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrMap;

        impl<'de> de::Visitor<'de> for StringOrMap {
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
                    "zlib" => Ok(Compression::zlib_default()),
                    "gzip" => Ok(Compression::gzip_default()),
                    "snappy" => Ok(Compression::Snappy),
                    _ => Err(Error::invalid_value(
                        de::Unexpected::Str(v),
                        &r#""none", "gzip", "zlib" or "snappy""#,
                    )),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut algorithm = None;
                let mut level = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "algorithm" => {
                            if algorithm.is_some() {
                                return Err(Error::duplicate_field("algorithm"));
                            }

                            algorithm = Some(map.next_value::<&str>()?);
                        }
                        "level" => {
                            if level.is_some() {
                                return Err(Error::duplicate_field("level"));
                            }

                            level = Some(map.next_value::<CompressionLevel>()?);
                        }
                        _ => return Err(Error::unknown_field(key, &["algorithm", "level"])),
                    };
                }

                let compression = match algorithm {
                    Some(value) => match value {
                        "none" => match level {
                            Some(_) => return Err(Error::unknown_field("level", &[])),
                            None => Compression::None,
                        },
                        "gzip" => Compression::Gzip(level.unwrap_or_default()),
                        "zlib" => Compression::Zlib(level.unwrap_or_default()),
                        "snappy" => match level {
                            Some(_) => return Err(Error::unknown_field("level", &[])),
                            None => Compression::Snappy,
                        },
                        algorithm => {
                            return Err(Error::unknown_variant(
                                algorithm,
                                &["none", "gzip", "zlib", "snappy"],
                            ))
                        }
                    },
                    None => {
                        return Err(Error::missing_field("algorithm"));
                    }
                };

                if let CompressionLevel::Value(level) = compression.compression_level() {
                    let max_level = compression.max_compression_level_value();
                    if level > max_level {
                        return Err(Error::custom(format!(
                            "invalid value '{level}', expected value in range [0, {max_level}]"
                        )));
                    }
                }

                Ok(compression)
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
        match self {
            Compression::None => serializer.serialize_str("none"),
            Compression::Gzip(level) => {
                if *level != CompressionLevel::Default {
                    let mut map = serializer.serialize_map(None)?;
                    map.serialize_entry("algorithm", "gzip")?;
                    map.serialize_entry("level", &level)?;
                    map.end()
                } else {
                    serializer.serialize_str("gzip")
                }
            }
            Compression::Zlib(level) => {
                if *level != CompressionLevel::Default {
                    let mut map = serializer.serialize_map(None)?;
                    map.serialize_entry("algorithm", "zlib")?;
                    map.serialize_entry("level", &level)?;
                    map.end()
                } else {
                    serializer.serialize_str("zlib")
                }
            }
            Compression::Zstd(level) => {
                if *level != CompressionLevel::Default {
                    let mut map = serializer.serialize_map(None)?;
                    map.serialize_entry("algorithm", "zstd")?;
                    map.serialize_entry("level", &level)?;
                    map.end()
                } else {
                    serializer.serialize_str("zstd")
                }
            }
            Compression::Snappy => serializer.serialize_str("snappy"),
        }
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
                Compression::Gzip(CompressionLevel::Fast),
            ),
            (
                "algorithm: gzip\nlevel: default",
                Compression::gzip_default(),
            ),
            (
                "algorithm: gzip\nlevel: best",
                Compression::Gzip(CompressionLevel::Best),
            ),
            (
                "algorithm: gzip\nlevel: 2",
                Compression::Gzip(CompressionLevel::Value(2)),
            ),
            (
                "algorithm: gzip\nlevel: 7",
                Compression::Gzip(CompressionLevel::Value(7)),
            ),
            (
                "algorithm: zlib",
                Compression::Zlib(CompressionLevel::Default),
            ),
            (
                "algorithm: zlib\nlevel: best",
                Compression::Zlib(CompressionLevel::Best),
            ),
            ("algorithm: snappy", Compression::Snappy),
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
                Compression::Gzip(CompressionLevel::default()),
            ),
            (
                r#"{"algorithm": "gzip", "level": "best"}"#,
                Compression::Gzip(CompressionLevel::Best),
            ),
            (
                r#"{
  "algorithm": "gzip",
  "level": 8
}"#,
                Compression::Gzip(CompressionLevel::Value(8)),
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
                r#"invalid value: string "b42", expected "none", "gzip", "zlib" or "snappy" at line 1 column 5"#,
            ),
            (
                r#"{"algorithm": "b42"}"#,
                r#"unknown variant `b42`, expected one of `none`, `gzip`, `zlib`, `snappy` at line 1 column 20"#,
            ),
            (
                r#"{"algorithm": "none", "level": "default"}"#,
                r#"unknown field `level`, there are no fields at line 1 column 41"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": -1}"#,
                r#"integer could not be converted to u32: out of range integral type conversion attempted at line 1 column 33"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": "good"}"#,
                r#"invalid value: string "good", expected "none", "fast", "best" or "default" at line 1 column 37"#,
            ),
            (
                r#"{"algorithm": "gzip", "level": {}}"#,
                r#"invalid type: map, expected unsigned number or string at line 1 column 33"#,
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
