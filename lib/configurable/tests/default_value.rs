#![allow(clippy::print_stdout)]

use std::fmt::Formatter;

use configurable::generate_config_with_schema;
use configurable::schema::generate_root_schema;
use configurable_derive::Configurable;
use serde::de::Unexpected;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Concurrency struct doc
#[derive(Configurable, Default)]
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

impl<'de> Deserialize<'de> for Concurrency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UsizeOrAdaptive;

        impl serde::de::Visitor<'_> for UsizeOrAdaptive {
            type Value = Concurrency;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r#"none, positive integer or "adaptive""#)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(serde::de::Error::invalid_value(
                        Unexpected::Signed(v),
                        &"positive integer",
                    ))
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v > 0 {
                    Ok(Concurrency::Fixed(v as usize))
                } else {
                    Err(serde::de::Error::invalid_value(
                        Unexpected::Unsigned(v),
                        &"positive integer",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v == "adaptive" {
                    Ok(Concurrency::Adaptive)
                } else if v.to_lowercase() == "none" {
                    Ok(Concurrency::None)
                } else {
                    Err(serde::de::Error::unknown_variant(v, &["adaptive", "none"]))
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

#[derive(Deserialize, Serialize, Configurable)]
struct Outer {
    #[serde(default)]
    concurrency: Concurrency,

    #[serde(default)]
    string: Option<String>,
}

#[test]
fn default_value() {
    let root_schema = generate_root_schema::<Outer>();

    let text = serde_json::to_string_pretty(&root_schema).unwrap();
    println!("{text}");

    let example = generate_config_with_schema(root_schema);
    println!("{example}");

    serde_yaml::from_str::<Outer>(&example).unwrap();
}

#[test]
fn none() {
    let value: Concurrency = Default::default();
    let value = serde_json::to_value(value).unwrap();
    println!("{value}")
}
