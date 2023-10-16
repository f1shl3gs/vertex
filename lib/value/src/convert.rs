use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Utc};

use crate::Value;

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
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

impl From<DateTime<Utc>> for Value {
    fn from(value: DateTime<Utc>) -> Self {
        Self::Timestamp(value)
    }
}

impl From<BTreeMap<String, Self>> for Value {
    fn from(value: BTreeMap<String, Self>) -> Self {
        Self::Object(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Self::Array(value)
    }
}

impl Value {
    /// Returns self as &BTreeMap<String, Value>, only if self is `Value::Object`
    pub fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        if let Self::Object(map) = self {
            Some(map)
        } else {
            None
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
