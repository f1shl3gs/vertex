use std::fmt::{Debug, Display, Formatter, write};

use serde::{de, ser, Serializer};
use serde::de::{Error, MapAccess};
use serde::Deserializer;
use serde::ser::{SerializeMap};
use serde_json::Value;

pub const GZIP_NONE: u32 = 0;
pub const GZIP_FAST: u32 = 1;
pub const GZIP_DEFAULT: u32 = 6;
pub const GZIP_BEST: u32 = 9;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Compression {
    None,
    Gzip(flate2::Compression),
}

impl Default for Compression {
    fn default() -> Self {
        Self::None
    }
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
            Self::Gzip(_) => Some("gzip")
        }
    }

    // TODO: maybe we don't need this
    pub const fn extension(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gzip(_) => "gz"
        }
    }
}

impl Display for Compression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::None => write!(f, "none"),
            Self::Gzip(ref level) => write!(f, "gzip({})", level.level())
        }
    }
}

impl<'de> serde::de::Deserialize<'de> for Compression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>
    {
        struct StringOrMap;

        impl<'de> serde::de::Visitor<'de> for StringOrMap {
            type Value = Compression;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
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
                    A: MapAccess<'de>
            {
                let mut algorithm = None;
                let mut level = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "algorithm" => {
                            if algorithm.is_some() {
                                return Err(de::Error::duplicate_field("algorithm"));
                            }

                            algorithm = Some(map.next_value::<&str>()?);
                        }
                        "level" => {
                            if level.is_some() {
                                return Err(de::Error::duplicate_field("level"));
                            }

                            level = Some(match map.next_value::<Value>()? {
                                Value::Number(level) => match level.as_u64() {
                                    Some(value) if value <= 9 => {
                                        flate2::Compression::new(value as u32)
                                    }
                                    Some(_) | None => {
                                        return Err(de::Error::invalid_value(
                                            de::Unexpected::Other(&level.to_string()),
                                            &"0, 1, 2, 3, 4, 5, 6, 7, 8, 9",
                                        ));
                                    }
                                },
                                Value::String(level) => match level.as_str() {
                                    "none" => flate2::Compression::none(),
                                    "fast" => flate2::Compression::fast(),
                                    "default" => flate2::Compression::default(),
                                    "best" => flate2::Compression::best(),
                                    level => {
                                        return Err(de::Error::invalid_value(
                                            de::Unexpected::Str(level),
                                            &r#""none", "fast", "default", "best""#,
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

                        _ => return Err(de::Error::unknown_field(
                            key,
                            &["algorithm", "level"],
                        ))
                    };
                }

                match algorithm.ok_or_else(|| de::Error::missing_field("algorithm"))? {
                    "none" => match level {
                        Some(_) => Err(de::Error::unknown_field("level", &[])),
                        None => Ok(Compression::None),
                    },
                    "gzip" => Ok(Compression::Gzip(level.unwrap_or_default())),
                    algorithm => Err(de::Error::unknown_variant(algorithm, &["none", "gzip"]))
                }
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}

impl ser::Serialize for Compression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer
    {
        use ser::Serializer;

        let mut map = serializer.serialize_map(None)?;

        match self {
            Compression::None => map.serialize_entry("algorithm", "none")?,
            Compression::Gzip(level) => {
                map.serialize_entry("algorithm", "gzip")?;
                match level.level() {
                    GZIP_NONE => map.serialize_entry("level", "none")?,
                    GZIP_FAST => map.serialize_entry("level", "fast")?,
                    GZIP_DEFAULT => map.serialize_entry("level", "default")?,
                    GZIP_BEST => map.serialize_entry("level", "best")?,
                    level => map.serialize_entry("level", &level)?,
                };
            }
        };

        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn deserialization() {
        #[derive(Debug, Deserialize, Serialize)]
        struct Config {
            compression: Compression
        }

        let valids = [
            // ("compression: none", Compression::None),
            ("compression:\n  algorithm: none", Compression::None),
            // (r#"algorithm: "gzip""#, Compression::gzip_default()),
            // ("algorithm: gzip\nlevel: fast", Compression::Gzip(flate2::Compression::fast())),
            // ("algorithm: gzip\nlevel: default", Compression::gzip_default()),
            // ("algorithm: gzip\nlevel: best", Compression::Gzip(flate2::Compression::best())),
        ];

        for (input, want) in valids {
            let conf: Config = serde_yaml::from_str(input).unwrap();
            assert_eq!(conf.compression, want, "input: {}", input);
        }
    }
}