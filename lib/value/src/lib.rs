mod convert;
mod crud;
mod display;
mod kind;
pub mod path;
mod serde;

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use bytes::Bytes;
use chrono::{DateTime, Utc};
pub use kind::Kind;
use path::ValuePath;

pub use path::{
    parse_target_path, parse_value_path, OwnedSegment, OwnedTargetPath, OwnedValuePath,
    PathParseError,
};

/// The main value type used in Vertex events.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Bytes - usually representing a UTF8 String,
    Bytes(Bytes),

    /// Integer
    Integer(i64),

    /// Float
    Float(f64),

    /// Boolean
    Boolean(bool),

    /// Timestamp with UTC
    Timestamp(DateTime<Utc>),

    /// Object
    Object(BTreeMap<String, Value>),

    /// Array
    Array(Vec<Value>),

    /// Null
    Null,
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Value::Bytes(b) => {
                state.write_u8(1);
                b.hash(state)
            }
            Value::Float(f) => {
                state.write_u8(2);
                f.to_bits().hash(state)
            }
            Value::Integer(i) => {
                state.write_u8(3);
                i.hash(state)
            }
            Value::Boolean(b) => {
                state.write_u8(4);
                b.hash(state)
            }
            Value::Timestamp(ts) => {
                state.write_u8(5);
                ts.hash(state)
            }
            Value::Object(obj) => {
                state.write_u8(6);
                obj.hash(state)
            }
            Value::Array(arr) => {
                state.write_u8(7);
                arr.hash(state)
            }
            Value::Null => state.write_u8(8),
        }
    }
}

impl Value {
    /// Returns a reference to a field value specified by a path iter.
    pub fn get<'a>(&self, path: impl ValuePath<'a>) -> Option<&Self> {
        crud::get(self, path.segment_iter())
    }

    /// Get a mutable borrow of the value by path.
    pub fn get_mut<'a>(&mut self, path: impl ValuePath<'a>) -> Option<&mut Self> {
        crud::get_mut(self, path.segment_iter())
    }

    /// Determine if the lookup is contained within the value.
    pub fn contains<'a>(&self, path: impl ValuePath<'a>) -> bool {
        self.get(path).is_some()
    }

    /// Returns a reference to a field value specified by a path iter.
    pub fn insert<'a>(
        &mut self,
        path: impl ValuePath<'a>,
        insert_value: impl Into<Self>,
    ) -> Option<Self> {
        let path_iter = path.segment_iter().peekable();
        crud::insert(self, (), path_iter, insert_value.into())
    }

    /// Removes field value specified by the given path and return its value.
    ///
    /// A special case worth mentioning: if there is a nested array and an item
    /// is removed from the middle of this array, then it is just replaced by
    /// `Value::Null`.
    pub fn remove<'a>(&mut self, path: impl ValuePath<'a>, prune: bool) -> Option<Self> {
        crud::remove(self, &(), path.segment_iter(), prune).map(|(prev, _is_empty)| prev)
    }
}

#[macro_export]
macro_rules! value {
    // arrays
    ([]) => ({
        $crate::Value::Array(vec![])
    });
    ([$($v:tt),+ $(,)?]) => ({
        let vec: Vec<$crate::Value> = vec![$($crate::value!($v)),+];
        $crate::Value::Array(vec)
    });

    // maps
    ({}) => ({
        $crate::Value::Object(::std::collections::BTreeMap::default())
    });
    ({$($($k1:literal)? $($k2:ident)?: $v:tt),+ $(,)?}) => ({
        let mut map = ::std::collections::BTreeMap::new();
        $(
             map.insert(String::from($($k1)? $(stringify!($k2))?), $crate::value!($v));
        )+

        $crate::Value::Object(map)
    });

    (null) => ({
        $crate::Value::Null
    });
    ($k:expr) => ({
        $crate::Value::from($k)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_macro() {
        assert_eq!(value!(1), Value::Integer(1));
        assert_eq!(value!(1.2), Value::Float(1.2));
        assert_eq!(value!("foo"), "foo".into());
        assert_eq!(value!(true), Value::Boolean(true));
        let ts = Utc::now();
        assert_eq!(value!(ts), Value::Timestamp(ts));
        assert_eq!(value!(null), Value::Null);

        let arr = value!([1, 2]);
        assert_eq!(arr, Value::Array(vec![value!(1), value!(2)]));

        let map = value!({"foo": "bar"});
        assert_eq!(map, {
            let mut map = BTreeMap::new();
            map.insert("foo".to_string(), Value::from("bar"));
            Value::Object(map)
        });

        let nested = value!({
            "foo": "bar",
            "arr": [1, 2],
            "map": {
                "key": "value"
            }
        });
        assert_eq!(nested, {
            let mut map = BTreeMap::new();
            map.insert("foo".to_string(), value!("bar"));
            map.insert("arr".to_string(), value!([1, 2]));

            let mut sub_map = BTreeMap::new();
            sub_map.insert("key".into(), "value".into());
            map.insert("map".into(), sub_map.into());

            Value::Object(map)
        })
    }
}
