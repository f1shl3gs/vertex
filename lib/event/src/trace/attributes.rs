use std::ops::Deref;

use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use typesize::TypeSize;

use super::{AnyValue, KeyValue};
use crate::tags::Key;

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct Attributes {
    key_values: Vec<KeyValue>,

    /// The number of attributes that were above the configured limit,
    /// and thus dropped
    dropped: usize,
}

impl Deref for Attributes {
    type Target = [KeyValue];

    fn deref(&self) -> &Self::Target {
        &self.key_values
    }
}

impl From<Vec<KeyValue>> for Attributes {
    fn from(key_values: Vec<KeyValue>) -> Self {
        Self {
            key_values,
            dropped: 0,
        }
    }
}

impl IntoIterator for Attributes {
    type Item = KeyValue;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.key_values.into_iter()
    }
}

impl TypeSize for Attributes {
    fn allocated_bytes(&self) -> usize {
        self.key_values.allocated_bytes()
    }
}

impl Serialize for Attributes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.key_values.len()))?;
        for KeyValue { key, value } in &self.key_values {
            map.serialize_key(key)?;
            map.serialize_value(value)?;
        }

        map.end()
    }
}

impl Attributes {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            key_values: Vec::with_capacity(capacity),
            dropped: 0,
        }
    }

    #[inline]
    pub fn insert(&mut self, key: impl Into<Key>, value: impl Into<AnyValue>) {
        self.key_values.push(KeyValue {
            key: key.into(),
            value: value.into(),
        })
    }

    pub fn remove(&mut self, key: &str) -> Option<AnyValue> {
        let index = self
            .key_values
            .iter()
            .position(|kv| kv.key.as_str() == key)?;

        Some(self.key_values.remove(index).value)
    }
}
