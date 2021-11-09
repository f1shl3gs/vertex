use std::collections::BTreeMap;
use std::convert::TryInto;

use serde::{Deserialize, Serialize, Serializer};
use bytes::{Bytes, BytesMut};
use chrono::{DateTime, SecondsFormat, Utc};

use crate::ByteSizeOf;


#[derive(PartialEq, PartialOrd, Debug, Clone, Deserialize)]
pub enum Value {
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
            Value::Bytes(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            Value::Float(f) => format!("{}", f),
            Value::Uint64(u) => format!("{}", u),
            Value::Int64(i) => format!("{}", i),
            Value::Array(arr) => serde_json::to_string(arr).expect("Cannot serialize array"),
            Value::Boolean(b) => format!("{}", b),
            Value::Map(m) => serde_json::to_string(m).expect("Cannot serialize map"),
            Value::Null => "<null>".to_string()
        }
    }

    pub fn as_bytes(&self) -> Bytes {
        match self {
            Value::Bytes(b) => b.clone(),
            Value::Float(f) => Bytes::from(format!("{}", f)),
            Value::Uint64(u) => Bytes::from(format!("{}", u)),
            Value::Int64(i) => Bytes::from(format!("{}", i)),
            Value::Array(arr) => {
                Bytes::from(serde_json::to_vec(arr).expect("Cannot serialize array"))
            }
            Value::Boolean(b) => Bytes::from(format!("{}", b)),
            Value::Map(m) => {
                Bytes::from(serde_json::to_vec(m).expect("Cannot serialize map"))
            }
            Value::Timestamp(ts) => Bytes::from(timestamp_to_string(ts)),
            Value::Null => Bytes::from("<null>")
        }
    }

    pub fn as_timestamp(&self) -> Option<&DateTime<Utc>> {
        match &self {
            Value::Timestamp(ts) => Some(ts),
            _ => None
        }
    }

    /// Merges `other` value into self.
    ///
    /// Will concatenate `Bytes` and overwrite the rest value kinds
    pub fn merge(&mut self, other: Value) {
        match (self, other) {
            (Value::Bytes(self_bytes), Value::Bytes(ref other)) => {
                let mut bytes = BytesMut::with_capacity(self_bytes.len() + other.len());
                bytes.extend_from_slice(&self_bytes[..]);
                bytes.extend_from_slice(&other[..]);
                *self_bytes = bytes.freeze();
            }

            (current, other) => *current = other
        }
    }

    /// Return if the node is empty, that is, it is an array or map with no items.
    pub fn is_empty(&self) -> bool {
        match &self {
            Value::Boolean(_)
            | Value::Bytes(_)
            | Value::Timestamp(_)
            | Value::Float(_)
            | Value::Uint64(_)
            | Value::Int64(_) => false,
            Value::Null => true,
            Value::Map(m) => m.is_empty(),
            Value::Array(arr) => arr.is_empty()
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
            Value::Bytes(_) | Value::Timestamp(_) => {
                serializer.serialize_str(&self.to_string_lossy())
            }
            Value::Boolean(b) => serializer.serialize_bool(*b),
            Value::Array(arr) => serializer.collect_seq(arr),
            Value::Map(m) => serializer.collect_map(m),
            Value::Null => serializer.serialize_none(),
            Value::Int64(v) => serializer.serialize_i64(*v)
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
            serde_json::Value::String(s) => Value::Bytes(s.into()),
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

/// Vector's basic error type, dynamically dispatched and safe to send across
/// threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

impl TryInto<serde_json::Value> for Value {
    type Error = Error;

    fn try_into(self) -> Result<serde_json::Value, Self::Error> {
        match self {
            Value::Boolean(v) => Ok(serde_json::Value::from(v)),
            Value::Bytes(v) => Ok(serde_json::Value::from(String::from_utf8(v.to_vec())?)),
            Value::Float(v) => Ok(serde_json::Value::from(v)),
            Value::Uint64(v) => Ok(serde_json::Value::from(v)),
            Value::Int64(v) => Ok(serde_json::Value::from(v)),
            Value::Array(v) => Ok(serde_json::to_value(v)?),
            Value::Map(v) => Ok(serde_json::to_value(v)?),
            Value::Timestamp(v) => Ok(serde_json::Value::from(timestamp_to_string(&v))),
            Value::Null => Ok(serde_json::Value::Null)
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

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Self::Int64(i as i64)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Uint64(u)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Int64(i)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::Bytes(s.into())
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::Bytes(Vec::from(s.as_bytes()).into())
    }
}

impl From<&[u8]> for Value {
    fn from(b: &[u8]) -> Self {
        Self::Bytes(Bytes::from(b.to_owned()))
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(ts: DateTime<Utc>) -> Self {
        Self::Timestamp(ts)
    }
}

impl From<Bytes> for Value {
    fn from(bytes: Bytes) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(map: BTreeMap<String, Value>) -> Self {
        Self::Map(map)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Self::Uint64(v as u64)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        value.into_iter()
            .map(::std::convert::Into::into)
            .collect::<Self>()
    }
}

impl FromIterator<Value> for Value {
    fn from_iter<T: IntoIterator<Item=Value>>(iter: T) -> Self {
        Value::Array(iter.into_iter().collect::<Vec<Value>>())
    }
}

impl FromIterator<(String, Value)> for Value {
    fn from_iter<T: IntoIterator<Item=(String, Value)>>(iter: T) -> Self {
        Value::Map(iter.into_iter().collect::<BTreeMap<String, Value>>())
    }
}

impl From<serde_yaml::Value> for Value {
    fn from(value: serde_yaml::Value) -> Self {
        match value {
            serde_yaml::Value::String(s) => Self::from(s),
            serde_yaml::Value::Number(n) => {
                if n.is_f64() {
                    Self::from(n.as_f64().unwrap())
                } else if n.is_i64() {
                    Self::from(n.as_i64().unwrap())
                } else {
                    Self::from(n.as_f64().unwrap())
                }
            }
            serde_yaml::Value::Null => Self::Null,
            serde_yaml::Value::Bool(b) => Self::from(b),
            serde_yaml::Value::Sequence(seq) => {
                let arr = seq.into_iter()
                    .map(Value::from)
                    .collect::<Vec<_>>();

                Self::from(arr)
            }
            serde_yaml::Value::Mapping(map) => {
                let mut fmap = BTreeMap::new();
                map.iter()
                    .map(|(k, v)| fmap.insert(
                        k.as_str().unwrap().to_owned(),
                        Self::from(v.clone()),
                    ));

                Self::from(fmap)
            }
        }
    }
}
/*
impl TryFrom<serde_yaml::Value> for Value {
    type Error = std::io::Error;

    fn try_from(value: serde_yaml::Value) -> Result<Self, Self::Error> {
        Ok(match value {
            serde_yaml::Value::String(s) => Self::from(s),
            serde_yaml::Value::Number(n) => {
                if n.is_f64() {
                    Self::from(n.as_f64().unwrap())
                } else if n.is_i64() {
                    Self::from(n.as_i64().unwrap())
                } else {
                    Self::from(n.as_f64().unwrap())
                }
            }
            serde_yaml::Value::Null => Self::Null,
            serde_yaml::Value::Bool(b) => Self::from(b),
            serde_yaml::Value::Sequence(seq) => {
                let arr = seq.into_iter()
                    .map(Value::try_from)
                    .collect::<Result<Vec<_>, std::io::Error>>()?;

                Self::from(arr)
            }
            serde_yaml::Value::Mapping(map) => {
                let mut fmap = BTreeMap::new();
                map.iter()
                    .map(|(k, v)| fmap.insert(
                        k.as_str().unwrap().to_owned(),
                        Self::try_from(v.clone()).unwrap(),
                    ));

                Self::from(fmap)
            }
        })
    }
}
*/
#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::path::Path;
    use super::*;

    fn parse_artifact(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        let mut f = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(err)
        };

        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        Ok(buf)
    }

    #[test]
    fn json_value_to_value_to_json_value() {
        const PREFIX: &str = "tests/data/fixtures/value";

        std::fs::read_dir(PREFIX)
            .unwrap()
            .for_each(|ent| match ent {
                Ok(type_name) => {
                    let path = type_name.path();
                    std::fs::read_dir(path)
                        .unwrap()
                        .for_each(|ent| match ent {
                            Ok(file) => {
                                let path = file.path();
                                let buf = parse_artifact(&path).unwrap();

                                let serde_value: serde_json::Value =
                                    serde_json::from_slice(&*buf).unwrap();
                                let value = Value::from(serde_value);

                                // Valid type
                                let expected_type = type_name
                                    .path()
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string();

                                let is_match = match value {
                                    Value::Boolean(_) => expected_type.eq("boolean"),
                                    Value::Int64(_) => expected_type.eq("integer"),
                                    Value::Uint64(_) => expected_type.eq("integer"),
                                    Value::Bytes(_) => expected_type.eq("bytes"),
                                    Value::Map(_) => expected_type.eq("map"),
                                    Value::Array(_) => expected_type.eq("array"),
                                    Value::Null => expected_type.eq("null"),
                                    _ => unreachable!("You need to add a new type handler here.")
                                };

                                assert!(
                                    is_match,
                                    "Typecheck failure. Wanted {}, got {:?}.",
                                    expected_type, value
                                );
                                let _v: serde_json::Value = value.try_into().unwrap();
                            }
                            _ => panic!("This test should never read Err test fixtures.")
                        });
                }
                _ => panic!("This test should never read Err type folders.")
            })
    }
}