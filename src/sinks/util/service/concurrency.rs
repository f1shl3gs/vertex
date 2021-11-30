use std::fmt::Formatter;

use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error, Unexpected, Visitor};


#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Concurrency {
    None,
    Adaptive,
    Fixed(usize),
}

impl Default for Concurrency {
    fn default() -> Self {
        Self::None
    }
}

impl Concurrency {
    pub const fn if_none(self, other: Self) -> Self {
        match self {
            Self::None => other,
            _ => self
        }
    }

    pub const fn parse_concurrency(&self, default: Self) -> Option<usize> {
        match self.if_none(default) {
            Concurrency::None | Concurrency::Adaptive => None,
            Concurrency::Fixed(limit) => Some(limit)
        }
    }
}

pub const fn concurrency_is_none(c: &Concurrency) -> bool {
    matches!(c, Concurrency::None)
}

impl<'de> Deserialize<'de> for Concurrency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        struct UsizeOrAdaptive;

        impl<'de> Visitor<'de> for UsizeOrAdaptive {
            type Value = Concurrency;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r#"positive integer or "adaptive""#)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: Error {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(serde::de::Error::invalid_value(
                        Unexpected::Signed(v),
                        &"positive integer"
                    ))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: Error {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(serde::de::Error::invalid_value(
                        Unexpected::Unsigned(v),
                        &"positive integer"
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                if v == "adaptive" {
                    Ok(Concurrency::Adaptive)
                } else {
                    Err(serde::de::Error::unknown_variant(v, &["adaptive"]))
                }
            }
        }

        deserializer.deserialize_any(UsizeOrAdaptive)
    }
}