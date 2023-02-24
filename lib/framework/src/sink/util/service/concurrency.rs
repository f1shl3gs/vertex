use std::fmt::Formatter;

use configurable::schema::{
    generate_const_string_schema, generate_number_schema, generate_one_of_schema, SchemaGenerator,
    SchemaObject,
};
use configurable::{Configurable, GenerateError};
use serde::de::{Error, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Configuration for outbound request concurrency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum Concurrency {
    /// A fixed concurrency of 1.
    ///
    /// Only one request can be outstanding at any given time.
    #[default]
    None,

    /// Concurrency will be managed by Vertex's [Adaptive Request Concurrency] feature.
    Adaptive,

    /// A fixed amount of concurrency will be allowed.
    Fixed(usize),
}

impl Configurable for Concurrency {
    fn generate_schema(_gen: &mut SchemaGenerator) -> Result<SchemaObject, GenerateError> {
        let schema = generate_one_of_schema(&[
            generate_const_string_schema("none".to_string()),
            generate_const_string_schema("adaptive".to_string()),
            generate_number_schema::<usize>(),
        ]);

        Ok(schema)
    }
}

impl Concurrency {
    pub const fn if_none(self, other: Self) -> Self {
        match self {
            Self::None => other,
            _ => self,
        }
    }

    pub const fn parse_concurrency(&self, default: Self) -> Option<usize> {
        match self.if_none(default) {
            Concurrency::None | Concurrency::Adaptive => None,
            Concurrency::Fixed(limit) => Some(limit),
        }
    }
}

pub const fn concurrency_is_none(c: &Concurrency) -> bool {
    matches!(c, Concurrency::None)
}

impl<'de> Deserialize<'de> for Concurrency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UsizeOrAdaptive;

        impl<'de> Visitor<'de> for UsizeOrAdaptive {
            type Value = Concurrency;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r#"positive integer or "adaptive""#)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(Error::invalid_value(
                        Unexpected::Signed(v),
                        &"positive integer",
                    ))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(Error::invalid_value(
                        Unexpected::Unsigned(v),
                        &"positive integer",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v == "adaptive" {
                    Ok(Concurrency::Adaptive)
                } else if v == "none" {
                    Ok(Concurrency::None)
                } else {
                    Err(Error::unknown_variant(v, &["adaptive"]))
                }
            }
        }

        deserializer.deserialize_any(UsizeOrAdaptive)
    }
}

impl Serialize for Concurrency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Concurrency::None => serializer.serialize_str("none"),
            Concurrency::Adaptive => serializer.serialize_str("adaptive"),
            Concurrency::Fixed(s) => serializer.serialize_u64(*s as u64),
        }
    }
}
