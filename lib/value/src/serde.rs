use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Formatter;

use bytes::Bytes;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::de::{Error, MapAccess, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Value;

impl Value {
    /// Converts self into a Bytes, using JSON for Map/Array.
    ///
    /// # Panics
    ///
    /// If map or array serialization fails.
    pub fn coerce_to_bytes(&self) -> Bytes {
        match self {
            Value::Bytes(b) => b.clone(),
            Value::Float(f) => Bytes::from(f.to_string()),
            Value::Integer(i) => Bytes::from(i.to_string()),
            Value::Boolean(b) => if *b { "true" } else { "false" }.into(),
            Value::Timestamp(ts) => timestamp_to_string(ts).into(),
            Value::Object(map) => serde_json::to_vec(map)
                .expect("Cannot serialize map")
                .into(),
            Value::Array(arr) => serde_json::to_vec(arr)
                .expect("Cannot serialize array")
                .into(),
            Value::Null => "<null>".into(),
        }
    }

    /// Converts self into a String representation, using JSON for Map/Array.
    ///
    /// # Panics
    ///
    /// If map or array serialization fails.
    pub fn to_string_lossy(&self) -> Cow<'_, str> {
        match self {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            Value::Float(f) => f.to_string().into(),
            Value::Integer(i) => i.to_string().into(),
            Value::Boolean(b) => if *b { "true" } else { "false" }.into(),
            Value::Timestamp(ts) => timestamp_to_string(ts).into(),
            Value::Object(map) => serde_json::to_string(map)
                .expect("Cannot serialize map")
                .into(),
            Value::Array(arr) => serde_json::to_string(arr)
                .expect("Cannot  serialize array")
                .into(),
            Value::Null => "<null>".into(),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Bytes(b) => serializer.serialize_str(String::from_utf8_lossy(b).as_ref()),
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::Integer(i) => serializer.serialize_i64(*i),
            Value::Boolean(b) => serializer.serialize_bool(*b),
            Value::Timestamp(ts) => serializer.serialize_str(&timestamp_to_string(ts)),
            Value::Object(o) => serializer.collect_map(o),
            Value::Array(a) => serializer.collect_seq(a),
            Value::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("any valid JSON value")
            }

            #[inline]
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.into())
            }

            #[inline]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.into())
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok((v as i64).into())
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::Float(v))
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                let b = Bytes::copy_from_slice(v.as_bytes());
                Ok(Value::Bytes(b))
            }

            #[inline]
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.into())
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::Null)
            }

            #[inline]
            fn visit_seq<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                while let Some(value) = visitor.next_element()? {
                    vec.push(value);
                }

                Ok(Value::Array(vec))
            }

            fn visit_map<A>(self, mut visitor: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut map = BTreeMap::new();
                while let Some((key, value)) = visitor.next_entry()? {
                    map.insert(key, value);
                }

                Ok(Value::Object(map))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

fn timestamp_to_string(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Bool(b) => Self::Boolean(b),
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    n.as_i64().expect("expect i64").into()
                } else if n.is_f64() {
                    n.as_f64().expect("expect f64").into()
                } else {
                    n.to_string().into()
                }
            }
            serde_json::Value::String(s) => Self::Bytes(Bytes::from(s)),
            serde_json::Value::Object(o) => {
                Self::Object(o.into_iter().map(|(k, v)| (k, Self::from(v))).collect())
            }
            serde_json::Value::Array(arr) => Self::Array(arr.into_iter().map(Self::from).collect()),
            serde_json::Value::Null => Self::Null,
        }
    }
}

impl From<&serde_json::Value> for Value {
    fn from(value: &serde_json::Value) -> Self {
        value.clone().into()
    }
}

impl TryInto<serde_json::Value> for Value {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        let value = match self {
            Self::Boolean(b) => serde_json::Value::from(b),
            Self::Integer(i) => serde_json::Value::from(i),
            Self::Float(f) => serde_json::Value::from(f),
            Self::Bytes(b) => serde_json::Value::from_iter(String::from_utf8(b.to_vec())),
            Self::Object(o) => serde_json::to_value(o)?,
            Self::Array(a) => serde_json::to_value(a)?,
            Self::Timestamp(ts) => serde_json::Value::from(timestamp_to_string(&ts)),
            Self::Null => serde_json::Value::Null,
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_value_to_vertex_value_to_json_value() {
        const FIXTURE_ROOT: &str = "tests/data/fixtures/value";

        let mut dirs = std::fs::read_dir(FIXTURE_ROOT).unwrap();
        while let Some(Ok(dir)) = dirs.next() {
            let type_name = dir
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let path = dir.path();
            let mut dirs = std::fs::read_dir(path).unwrap();
            while let Some(Ok(entry)) = dirs.next() {
                let data = std::fs::read_to_string(entry.path()).unwrap();

                let serde_value: serde_json::Value =
                    serde_json::from_slice(data.as_bytes()).unwrap();
                let vertex_value = Value::from(serde_value);

                // Validate type
                let expected_type = type_name.to_string();
                let is_match = match vertex_value {
                    Value::Boolean(_) => expected_type == "boolean",
                    Value::Integer(_) => expected_type == "integer",
                    Value::Bytes(_) => expected_type == "bytes",
                    Value::Array(_) => expected_type == "array",
                    Value::Object(_) => expected_type == "map",
                    Value::Null => expected_type == "null",
                    _ => unreachable!("You need to add a new type handler here"),
                };

                assert!(
                    is_match,
                    "Typecheck failure. Wanted {expected_type}, got {vertex_value:?}"
                );
                let _value: serde_json::Value = vertex_value.try_into().unwrap();
            }
        }
    }
}
