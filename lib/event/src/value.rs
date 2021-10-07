use std::collections::BTreeMap;
use serde::{Deserialize, Serialize, Serializer};
use bytes::{Bytes};
use chrono::{DateTime, SecondsFormat, Utc};
use crate::ByteSizeOf;

#[derive(PartialEq, PartialOrd, Debug, Clone, Deserialize)]
pub enum Value {
    String(String),
    Bytes(Bytes),
    Float(f64),
    Uint64(u64),
    Int64(i64),
    Array(Vec<Value>),
    Boolean(bool),
    Map(BTreeMap<String, Value>),
    Timestamp(DateTime<Utc>),
    Null,
}

impl Value {
    pub fn to_string_lossy(&self) -> String {
        match self {
            Value::Timestamp(ts) => timestamp_to_string(ts),
            _ => todo!()
        }
    }
}

impl ByteSizeOf for Value {
    fn allocated_bytes(&self) -> usize {
        match self {
            Value::Bytes(bytes) => bytes.len(),
            Value::Map(map) => map
                .iter()
                .fold(0, |acc, (k, v)| acc + k.len() + v.size_of()),
            Value::Array(arr) => arr.iter().fold(0, |acc, v| acc + v.size_of()),
            _ => 0
        }
    }
}

fn timestamp_to_string(timestamp: &DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer
    {
        match &self {
            Value::Uint64(u) => serializer.serialize_u64(*u),
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::String(s) => serializer.serialize_str(s),
            Value::Timestamp(_) => serializer.serialize_str(&self.to_string_lossy()),
            Value::Boolean(b) => serializer.serialize_bool(*b),
            Value::Array(arr) => serializer.collect_seq(arr),
            Value::Map(m) => serializer.collect_map(m),
            _ => todo!()
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(json_value: serde_json::Value) -> Self {
        match json_value {
            serde_json::Value::Bool(b) => b.into(),
            serde_json::Value::Number(n) => {
                let float_or_byte = || {
                    n.as_f64()
                        .map_or_else(|| Value::Bytes(n.to_string().into()), Value::Float)
                };

                n.as_i64().map_or_else(float_or_byte, Value::Int64)
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Object(obj) => Value::Map(
                obj.into_iter()
                    .map(|(key, value)| (key, Value::from(value)))
                    .collect(),
            ),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Null => Value::Null
        }
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<u8> for Value {
    fn from(u: u8) -> Self {
        Self::Uint64(u as u64)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Uint64(u)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Self {
        Self::String(s.into())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_to_fields() {
        let text = r#"{"_SYSTEMD_UNIT":"sysinit.target","MESSAGE":"System Initialization","__CURSOR":"1","_SOURCE_REALTIME_TIMESTAMP":"1578529839140001","PRIORITY":"6"}"#;
        let v: BTreeMap<String, Value> = serde_json::from_str(text).unwrap();
        println!("{:?}", v);
    }
}