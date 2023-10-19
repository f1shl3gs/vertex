use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Utc};

use crate::Value;

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Integer(value as i64)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Bytes(Bytes::copy_from_slice(value.as_bytes()))
    }
}

impl From<Bytes> for Value {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

impl From<DateTime<Utc>> for Value {
    fn from(value: DateTime<Utc>) -> Self {
        Self::Timestamp(value)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(value: BTreeMap<String, Self>) -> Self {
        Self::Object(value)
    }
}

impl FromIterator<Self> for Value {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        Self::Array(iter.into_iter().collect::<Vec<Self>>())
    }
}

impl<T: Into<Self>> From<Option<T>> for Value {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

impl<T: Into<Self>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        value.into_iter().map(Into::into).collect::<Self>()
    }
}

impl Value {
    /// Returns self as &DateTime<Utc>, only if self is Value::Timestamp
    pub fn as_timestamp(&self) -> Option<&DateTime<Utc>> {
        match &self {
            Self::Timestamp(ts) => Some(ts),
            _ => None,
        }
    }

    /// Returns self as &BTreeMap<String, Value>, only if self is `Value::Object`
    pub fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        match &self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Returns self as `&mut BTreeMap<String, Value>`, only if self is `Value::Object`
    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Self>> {
        match self {
            Self::Object(v) => Some(v),
            _ => None,
        }
    }

    /// Returns self as &Bytes, only if self is Value::Bytes
    pub fn as_bytes(&self) -> Option<&Bytes> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Returns self as a `Vec<Value>`.
    ///
    /// # Panic
    ///
    /// This function will panic if self is anything other than `Value::Array`
    pub fn as_array_unwrap(&self) -> &[Self] {
        if let Self::Array(ref array) = self {
            array
        } else {
            panic!("Tried to call `Value::as_array_unwrap` on a non-array value")
        }
    }
}
