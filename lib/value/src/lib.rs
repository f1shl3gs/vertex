mod convert;
mod crud;
mod display;
pub mod path;
mod serde;

use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use path::ValuePath;

pub use path::{parse_value_path, OwnedTargetPath, OwnedValuePath};

/// The main value type used in Vertex events.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Bytes - usually representing a UTF8 String,
    Bytes(Bytes),

    /// Float
    Float(f64),

    /// Integer
    Integer(i64),

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
macro_rules! map_value {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map = std::collections::BTreeMap::<String, $crate::Value>::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        fields!{$($x => $y),*}
    );
    () => ({
        std::collections::BTreeMap::<String, $crate::Value>::new();
    })
}
