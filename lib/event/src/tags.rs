use std::borrow::Cow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

/// Key used for metric `AttributeSet`s and trace `Span` attributes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Key(Cow<'static, str>);

impl Key {
    /// Create a new `Key`.
    pub fn new<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Key(value.into())
    }

    /// Create a new const `Key`.
    pub const fn from_static_str(value: &'static str) -> Self {
        Key(Cow::Borrowed(value))
    }

    /// Returns a reference to the underlying key name
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl From<&'static str> for Key {
    /// Convert a `&str` to a `Key`.
    fn from(key_str: &'static str) -> Self {
        Key(Cow::from(key_str))
    }
}

impl From<Cow<'static, str>> for Key {
    fn from(value: Cow<'static, str>) -> Self {
        Self(value)
    }
}

impl From<String> for Key {
    /// Convert a `String` to a `Key`.
    fn from(string: String) -> Self {
        Key(Cow::from(string))
    }
}

impl From<Key> for String {
    /// Converts `Key` instances into `String`.
    fn from(key: Key) -> Self {
        key.0.into_owned()
    }
}

impl From<&String> for Key {
    fn from(s: &String) -> Self {
        Key(Cow::from(s.to_string()))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(fmt)
    }
}

/// Array of homogeneous values
#[derive(Clone, Debug, Deserialize, Serialize, PartialOrd)]
#[serde(untagged)]
pub enum Array {
    /// Array of bools
    Bool(Vec<bool>),
    /// Array of integers
    I64(Vec<i64>),
    /// Array of floats
    F64(Vec<f64>),
    /// Array of strings
    String(Vec<Cow<'static, str>>),
}

impl PartialEq<Array> for Array {
    fn eq(&self, other: &Array) -> bool {
        match (self, other) {
            (Array::Bool(a), Array::Bool(b)) => a.eq(b),
            (Array::I64(a), Array::I64(b)) => a.eq(b),
            (Array::F64(a), Array::F64(b)) => a.eq(b),
            (Array::String(a), Array::String(b)) => a.eq(b),
            _ => false,
        }
    }
}

impl Hash for Array {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self);

        match self {
            Array::Bool(b) => b.hash(state),
            Array::I64(i) => i.hash(state),
            Array::F64(f) => {
                // This hashes floats with the following rules:
                // * NaNs hash as equal (covered by above discriminant hash)
                // * Positive and negative infinity has to different values
                // * -0 and +0 hash to different values
                // * otherwise transmute to u64 and hash
                for v in f.iter() {
                    if v.is_finite() {
                        v.is_sign_negative().hash(state);
                        let trunc: u64 = v.trunc().to_bits();
                        trunc.hash(state);
                    } else if !v.is_nan() {
                        v.is_sign_negative().hash(state);
                    } //else covered by discriminant hash
                }
            }
            Array::String(s) => s.hash(state),
        }
    }
}

impl fmt::Display for Array {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Array::Bool(values) => display_array_str(values, fmt),
            Array::I64(values) => display_array_str(values, fmt),
            Array::F64(values) => display_array_str(values, fmt),
            Array::String(values) => {
                write!(fmt, "[")?;
                for (i, t) in values.iter().enumerate() {
                    if i > 0 {
                        write!(fmt, ",")?;
                    }
                    write!(fmt, "{:?}", t)?;
                }
                write!(fmt, "]")
            }
        }
    }
}

fn display_array_str<T: fmt::Display>(slice: &[T], fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(fmt, "[")?;
    for (i, t) in slice.iter().enumerate() {
        if i > 0 {
            write!(fmt, ",")?;
        }
        write!(fmt, "{}", t)?;
    }
    write!(fmt, "]")
}

macro_rules! into_array {
    ($(($t:ty, $val:expr),)+) => {
        $(
            impl From<$t> for Array {
                fn from(t: $t) -> Self {
                    $val(t)
                }
            }
        )+
    }
}

into_array!(
    (Vec<bool>, Array::Bool),
    (Vec<i64>, Array::I64),
    (Vec<f64>, Array::F64),
    (Vec<Cow<'static, str>>, Array::String),
);

impl From<Vec<String>> for Array {
    fn from(ss: Vec<String>) -> Self {
        let ss = ss.into_iter().map(Into::into).collect::<Vec<_>>();

        Self::String(ss)
    }
}

/// Value types for use in `KeyValue` pairs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialOrd)]
#[serde(untagged)]
pub enum Value {
    /// bool values
    Bool(bool),
    /// i64 values
    I64(i64),
    /// f64 values
    F64(f64),
    /// String values
    String(Cow<'static, str>),
    /// Array of homogeneous values
    Array(Array),
}

impl Eq for Value {}

impl PartialEq<Value> for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a.eq(b),
            (Value::I64(a), Value::I64(b)) => a.eq(b),
            (Value::F64(a), Value::F64(b)) => {
                // This compares floats with the following rules:
                // * NaNs compares as equal
                // * Positive and negative infinity are not equal
                // * -0 and +0 are not equal
                // * Floats will compare using truncated portion
                if a.is_sign_negative() == b.is_sign_negative() {
                    if a.is_finite() && b.is_finite() {
                        a.trunc().eq(&b.trunc())
                    } else {
                        a.is_finite() == b.is_finite()
                    }
                } else {
                    false
                }
            }
            (Value::String(a), Value::String(b)) => a.eq(b),
            (Value::Array(a), Value::Array(b)) => a.eq(b),
            _ => false,
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        match self {
            Value::Bool(b) => b.hash(state),
            Value::I64(i) => i.hash(state),
            Value::F64(f) => {
                // This hashes floats with the following rules:
                // * NaNs hash as equal (covered by above discriminant hash)
                // * Positive and negative infinity has to different values
                // * -0 and +0 hash to different values
                // * otherwise transmute to u64 and hash
                if f.is_finite() {
                    f.is_sign_negative().hash(state);
                    let trunc: u64 = f.trunc().to_bits();
                    trunc.hash(state);
                } else if !f.is_nan() {
                    f.is_sign_negative().hash(state);
                } // else covered by discriminant hash
            }
            Value::String(s) => s.hash(state),
            Value::Array(a) => a.hash(state),
        }
    }
}

impl Value {
    /// String representation of the `Value`
    ///
    /// This will allocate iff the underlying value is not a `String`.
    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            Value::Bool(v) => format!("{}", v).into(),
            Value::I64(v) => format!("{}", v).into(),
            Value::F64(v) => format!("{}", v).into(),
            Value::String(v) => Cow::Borrowed(v.as_ref()),
            Value::Array(v) => format!("{}", v).into(),
        }
    }
}

macro_rules! from_values {
   (
        $(
            ($ty:ty, $val:expr);
        )+
   ) => {
       $(
           impl From<$ty> for Value {
               fn from(ty: $ty) -> Self {
                   $val(ty)
               }
           }
       )+
   }
}

from_values!(
    (bool, Value::Bool);
    (i64, Value::I64);
    (f64, Value::F64);
    (Cow<'static, str>, Value::String);
    (Array, Value::Array);
);

impl From<i32> for Value {
    fn from(v: i32) -> Value {
        Value::I64(v as i64)
    }
}

impl From<&'static str> for Value {
    /// Convenience method for creating a `Value` from a `&'static str`.
    fn from(s: &'static str) -> Self {
        Value::String(s.into())
    }
}

impl From<String> for Value {
    /// Convenience method for creating a `Value` from a `String`.
    fn from(s: String) -> Self {
        Value::String(s.into())
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Self {
        s.to_string().into()
    }
}

impl From<Vec<String>> for Value {
    fn from(ss: Vec<String>) -> Self {
        let array: Array = ss.into();
        Self::Array(array)
    }
}

impl From<Vec<&str>> for Value {
    fn from(ss: Vec<&str>) -> Self {
        let array: Array = ss
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .into();

        Self::Array(array)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => fmt.write_fmt(format_args!("{}", v)),
            Value::I64(v) => fmt.write_fmt(format_args!("{}", v)),
            Value::F64(v) => fmt.write_fmt(format_args!("{}", v)),
            Value::String(v) => fmt.write_fmt(format_args!("{}", v)),
            Value::Array(v) => fmt.write_fmt(format_args!("{}", v)),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Hash, PartialEq, PartialOrd, Eq)]
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
    pub fn contains_key(&self, key: impl Into<Key>) -> bool {
        self.0.contains_key(&(key.into()))
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

                k.0.len() + vl
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
        let raw = r##"{"bool":true,"bool_array":[true,false],"float64":2.0,"float_array":[1.0,2.0],"int64":1,"int_array":[1,2],"string":"str","string_array":["foo","bar"]}"##;
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
