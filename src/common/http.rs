use std::fmt::Formatter;

use configurable::schema::{SchemaGenerator, SchemaObject};
use configurable::Configurable;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug)]
pub enum HttpParamKind {
    Glob(glob::Pattern),
    Exact(String),
}

impl<'de> Deserialize<'de> for HttpParamKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct HttpParamKindVisitor;

        impl Visitor<'_> for HttpParamKindVisitor {
            type Value = HttpParamKind;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value.contains('*') {
                    let pattern =
                        glob::Pattern::new(value).map_err(|err| serde::de::Error::custom(err))?;

                    Ok(HttpParamKind::Glob(pattern))
                } else {
                    Ok(HttpParamKind::Exact(value.to_string()))
                }
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if value.contains('*') {
                    let pattern =
                        glob::Pattern::new(&value).map_err(|err| serde::de::Error::custom(err))?;

                    Ok(HttpParamKind::Glob(pattern))
                } else {
                    Ok(HttpParamKind::Exact(value))
                }
            }
        }

        deserializer.deserialize_any(HttpParamKindVisitor)
    }
}

impl Serialize for HttpParamKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            HttpParamKind::Glob(pattern) => serializer.serialize_str(pattern.as_str()),
            HttpParamKind::Exact(value) => serializer.serialize_str(value.as_str()),
        }
    }
}

impl Configurable for HttpParamKind {
    fn generate_schema(gen: &mut SchemaGenerator) -> SchemaObject {
        gen.subschema_for::<String>()
    }
}
