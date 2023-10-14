mod key;
mod value;

use std::collections::btree_map::{Entry, Keys};
use std::collections::BTreeMap;
use std::hash::Hash;

pub use key::Key;
use measurable::ByteSizeOf;
use serde::Serialize;
pub use value::{Array, Value};

#[derive(Clone, Debug, Default, Serialize, Hash, PartialEq, PartialOrd, Eq)]
pub struct Tags(BTreeMap<Key, Value>);

impl Tags {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Returns a front-to-back iterator.
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.0.iter())
    }

    pub fn insert(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.0.insert(key.into(), value.into());
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub fn remove(&mut self, key: &Key) -> Option<Value> {
        self.0.remove(key)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.0.get(key)
    }

    #[inline]
    pub fn entry(&mut self, key: impl Into<Key>) -> Entry<Key, Value> {
        self.0.entry(key.into())
    }

    #[inline]
    pub fn contains_key(&self, key: &Key) -> bool {
        self.0.contains_key(key)
    }

    pub fn keys(&self) -> Keys<'_, Key, Value> {
        self.0.keys()
    }

    #[must_use]
    pub fn with(&self, key: impl Into<Key>, value: impl Into<Value>) -> Self {
        let mut new = self.clone();
        new.0.insert(key.into(), value.into());
        new
    }
}

impl FromIterator<(Key, Value)> for Tags {
    fn from_iter<T: IntoIterator<Item = (Key, Value)>>(iter: T) -> Self {
        let mut attrs = Tags::default();
        iter.into_iter().for_each(|(k, v)| attrs.insert(k, v));

        attrs
    }
}

impl From<BTreeMap<String, String>> for Tags {
    fn from(map: BTreeMap<String, String>) -> Self {
        let map = map
            .into_iter()
            .map(|(k, v)| (Key::from(k), Value::from(v)))
            .collect();

        Self(map)
    }
}

impl ByteSizeOf for Tags {
    fn allocated_bytes(&self) -> usize {
        self.0
            .iter()
            .map(|(k, v)| {
                let vl = match v {
                    Value::String(s) => s.len(),
                    _ => 0,
                };

                k.allocated_bytes() + vl
            })
            .sum()
    }
}

impl<T> std::ops::Index<T> for Tags
where
    T: AsRef<str>,
{
    type Output = Value;

    fn index(&self, index: T) -> &Self::Output {
        let key = Key::new(index.as_ref().to_owned());
        self.0.get(&key).unwrap()
    }
}

/// An owned iterator over the entries of a `Attributes`.
#[derive(Debug)]
pub struct IntoIter(std::collections::btree_map::IntoIter<Key, Value>);

impl Iterator for IntoIter {
    type Item = (Key, Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl IntoIterator for Tags {
    type Item = (Key, Value);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

impl<'a> IntoIterator for &'a Tags {
    type Item = (&'a Key, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.0.iter())
    }
}

/// An iterator over the entries of an `Attributes`.
#[derive(Debug)]
pub struct Iter<'a>(std::collections::btree_map::Iter<'a, Key, Value>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Key, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub fn skip_serializing_if_empty(attrs: &Tags) -> bool {
    attrs.0.is_empty()
}

#[macro_export]
macro_rules! tags {
    // Done without trailing comma
    ( $($x:expr => $y:expr),* ) => ({
        let mut _tags = $crate::tags::Tags::new();
        $(
            _tags.insert($x, $y);
        )*
        _tags
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        tags!{$($x => $y),*}
    );
}

#[macro_export]
macro_rules! btreemap {
    // Done without trailing comma
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        btreemap!{$($x => $y),*}
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_value() {
        let mut map = BTreeMap::new();
        map.insert("bool", Value::Bool(true));
        map.insert("int64", Value::I64(1));
        map.insert("float64", Value::F64(2.0));
        map.insert("string", Value::String("str".into()));
        map.insert("bool_array", Value::Array(Array::Bool(vec![true, false])));
        map.insert("int_array", Value::Array(Array::I64(vec![1, 2])));
        map.insert("float_array", Value::Array(Array::F64(vec![1.0, 2.0])));
        map.insert(
            "string_array",
            Value::Array(Array::String(vec!["foo".into(), "bar".into()])),
        );

        serde_json::to_string(&map).unwrap();
    }

    #[test]
    fn deserialize_value() {
        let raw = r#"{"bool":true,"bool_array":[true,false],"float64":2.0,"float_array":[1.0,2.0],"int64":1,"int_array":[1,2],"string":"str","string_array":["foo","bar"]}"#;
        let map: BTreeMap<String, Value> = serde_json::from_str(raw).unwrap();

        assert_eq!(map.get("bool").unwrap(), &Value::Bool(true));
        assert_eq!(map.get("int64").unwrap(), &Value::I64(1));
        assert_eq!(map.get("float64").unwrap(), &Value::F64(2.0));
        assert_eq!(map.get("string").unwrap(), &Value::String("str".into()));
        assert_eq!(
            map.get("bool_array").unwrap(),
            &Value::Array(Array::Bool(vec![true, false]))
        );
        assert_eq!(
            map.get("int_array").unwrap(),
            &Value::Array(Array::I64(vec![1, 2]))
        );
        assert_eq!(
            map.get("float_array").unwrap(),
            &Value::Array(Array::F64(vec![1.0, 2.0]))
        );
        assert_eq!(
            map.get("string_array").unwrap(),
            &Value::Array(Array::String(vec!["foo".into(), "bar".into()]))
        );
    }
}
