mod crud;
pub mod keys;

use std::collections::BTreeMap;
use std::convert::TryInto;

use bytes::{Bytes, BytesMut};
use chrono::{DateTime, SecondsFormat, Utc};
pub use crud::ValueCollection;
use lookup::Path;
use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize, Serializer};

#[derive(PartialEq, PartialOrd, Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Bytes(Bytes),
    Float(f64),
    Int64(i64),
    Boolean(bool),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
    Timestamp(DateTime<Utc>),
    Null,
}

impl Value {
    pub fn to_string_lossy(&self) -> String {
        match self {
            Value::Timestamp(ts) => timestamp_to_string(ts),
            Value::Bytes(bytes) => String::from_utf8_lossy(bytes).into_owned(),
            Value::Float(f) => format!("{f}"),
            Value::Int64(i) => format!("{i}"),
            Value::Array(arr) => serde_json::to_string(arr).expect("Cannot serialize array"),
            Value::Boolean(b) => format!("{b}"),
            Value::Object(m) => serde_json::to_string(m).expect("Cannot serialize map"),
            Value::Null => "<null>".to_string(),
        }
    }

    pub fn coerce_to_bytes(&self) -> Bytes {
        match self {
            Value::Bytes(b) => b.clone(),
            Value::Float(f) => f.to_string().into(),
            Value::Int64(i) => i.to_string().into(),
            Value::Boolean(b) => b.to_string().into(),
            Value::Timestamp(ts) => timestamp_to_string(ts).into(),
            Value::Array(arr) => serde_json::to_vec(arr)
                .expect("Cannot serialize array")
                .into(),
            Value::Object(map) => serde_json::to_vec(map)
                .expect("Cannot serialize array")
                .into(),
            Value::Null => Bytes::from("<null>"),
        }
    }

    pub fn as_bytes(&self) -> Bytes {
        match self {
            Value::Bytes(b) => b.clone(),
            Value::Float(f) => Bytes::from(format!("{f}")),
            Value::Int64(i) => Bytes::from(format!("{i}")),
            Value::Array(arr) => {
                Bytes::from(serde_json::to_vec(arr).expect("Cannot serialize array"))
            }
            Value::Boolean(b) => Bytes::from(format!("{b}")),
            Value::Object(m) => Bytes::from(serde_json::to_vec(m).expect("Cannot serialize map")),
            Value::Timestamp(ts) => Bytes::from(timestamp_to_string(ts)),
            Value::Null => Bytes::from("<null>"),
        }
    }

    pub fn as_timestamp(&self) -> Option<&DateTime<Utc>> {
        match &self {
            Value::Timestamp(ts) => Some(ts),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Returns self as a mutable `BTreeMap<String, Value>`.
    ///
    /// # Panics
    ///
    /// This function will panic if self is anything other than `Value::Object`.
    pub fn as_object_mut_unwrap(&mut self) -> &mut BTreeMap<String, Self> {
        match self {
            Value::Object(ref mut m) => m,
            _ => panic!("Tried to call `Value::as_map` on a non-map value"),
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

            (current, other) => *current = other,
        }
    }

    /// Return if the node is empty, that is, it is an array or map with no items.
    pub fn is_empty(&self) -> bool {
        match &self {
            Value::Boolean(_)
            | Value::Bytes(_)
            | Value::Timestamp(_)
            | Value::Float(_)
            | Value::Int64(_) => false,
            Value::Null => true,
            Value::Object(m) => m.is_empty(),
            Value::Array(arr) => arr.is_empty(),
        }
    }

    /// Returns a reference to a field value specified by a path iter.
    #[allow(clippy::needless_pass_by_value)]
    pub fn insert<'a>(&mut self, path: impl Path<'a>, value: impl Into<Self>) -> Option<Self> {
        let value = value.into();
        let path_iter = path.segment_iter().peekable();

        crud::insert(self, (), path_iter, value)
    }

    /// Removes field value specified by the given path and return its value.
    ///
    /// A special case worth mentioning: if there is a nested array and an item is removed
    /// from the middle of this array, then it is just replaced by `Value::Null`
    #[allow(clippy::needless_pass_by_value)]
    pub fn remove<'a>(&mut self, path: impl Path<'a>, prune: bool) -> Option<Self> {
        crud::remove(self, &(), path.segment_iter(), prune).map(|(prev, _is_empty)| prev)
    }

    /// Returns a reference to a field value specified by a path iter.
    #[allow(clippy::needless_pass_by_value)]
    pub fn get<'a>(&self, path: impl Path<'a>) -> Option<&Self> {
        crud::get(self, path.segment_iter())
    }

    /// Get a mutable borrow of the value by path
    #[allow(clippy::needless_pass_by_value)]
    pub fn get_mut<'a>(&mut self, path: impl Path<'a>) -> Option<&mut Self> {
        crud::get_mut(self, path.segment_iter())
    }

    /// Determine if the lookup is contained within the value.
    pub fn contains<'a>(&self, path: impl Path<'a>) -> bool {
        self.get(path).is_some()
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Object(BTreeMap::new())
    }
}

impl ByteSizeOf for Value {
    fn allocated_bytes(&self) -> usize {
        match self {
            Value::Bytes(bytes) => bytes.len(),
            Value::Object(map) => map
                .iter()
                .fold(0, |acc, (k, v)| acc + k.len() + v.size_of()),
            Value::Array(arr) => arr.iter().fold(0, |acc, v| acc + v.size_of()),
            _ => 0,
        }
    }
}

fn timestamp_to_string(timestamp: &DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self {
            Value::Float(f) => serializer.serialize_f64(*f),
            Value::Bytes(_) | Value::Timestamp(_) => {
                serializer.serialize_str(&self.to_string_lossy())
            }
            Value::Boolean(b) => serializer.serialize_bool(*b),
            Value::Array(arr) => serializer.collect_seq(arr),
            Value::Object(m) => serializer.collect_map(m),
            Value::Null => serializer.serialize_none(),
            Value::Int64(v) => serializer.serialize_i64(*v),
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
            serde_json::Value::Object(obj) => Value::Object(
                obj.into_iter()
                    .map(|(key, value)| (key, Value::from(value)))
                    .collect(),
            ),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Null => Value::Null,
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
            Value::Int64(v) => Ok(serde_json::Value::from(v)),
            Value::Array(v) => Ok(serde_json::to_value(v)?),
            Value::Object(v) => Ok(serde_json::to_value(v)?),
            Value::Timestamp(v) => Ok(serde_json::Value::from(timestamp_to_string(&v))),
            Value::Null => Ok(serde_json::Value::Null),
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
        Self::Int64(u as i64)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Self::Int64(i as i64)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Int64(u as i64)
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
        Self::Object(map)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<u32> for Value {
    fn from(v: u32) -> Self {
        Self::Int64(v as i64)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        value
            .into_iter()
            .map(::std::convert::Into::into)
            .collect::<Self>()
    }
}

impl FromIterator<Value> for Value {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Value::Array(iter.into_iter().collect::<Vec<Value>>())
    }
}

impl FromIterator<(String, Value)> for Value {
    fn from_iter<T: IntoIterator<Item = (String, Value)>>(iter: T) -> Self {
        Value::Object(iter.into_iter().collect::<BTreeMap<String, Value>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::path::Path;

    fn parse_artifact(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        let mut f = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(err) => return Err(err),
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
                    std::fs::read_dir(path).unwrap().for_each(|ent| match ent {
                        Ok(file) => {
                            let path = file.path();
                            let buf = parse_artifact(path).unwrap();

                            let serde_value: serde_json::Value =
                                serde_json::from_slice(&buf).unwrap();
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
                                Value::Bytes(_) => expected_type.eq("bytes"),
                                Value::Object(_) => expected_type.eq("map"),
                                Value::Array(_) => expected_type.eq("array"),
                                Value::Null => expected_type.eq("null"),
                                _ => unreachable!("You need to add a new type handler here."),
                            };

                            assert!(
                                is_match,
                                "Typecheck failure. Wanted {}, got {:?}.",
                                expected_type, value
                            );
                            let _v: serde_json::Value = value.try_into().unwrap();
                        }
                        _ => panic!("This test should never read Err test fixtures."),
                    });
                }
                _ => panic!("This test should never read Err type folders."),
            });
    }
}
