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
    OwnedSegment, OwnedTargetPath, OwnedValuePath, PathParseError, parse_target_path,
    parse_value_path,
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

impl typesize::TypeSize for Value {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        match self {
            Self::Array(a) => a.iter().fold(0, |acc, item| acc + item.size_of()),
            Self::Bytes(b) => b.allocated_bytes(),
            Self::Object(o) => o.allocated_bytes(),
            _ => 0,
        }
    }
}

impl Value {
    #[inline]
    pub fn object() -> Self {
        Self::Object(BTreeMap::new())
    }

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
    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a vec![...]
    // of the elements.
    //
    // Must be invoked as: value!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        vec![$($elems,)*]
    };

    // Done without trailing comma.
    (@array [$($elems:expr),*]) => {
        vec![$($elems),*]
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!(null)] $($rest)*)
    };

    // Next element is `true`.
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!(true)] $($rest)*)
    };

    // Next element is `false`.
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!(false)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)* $crate::value!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::value!(@array [$($elems,)* $crate::value!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::value!(@array [$($elems,)*] $($rest)*)
    };

    // Unexpected token after most recent element.
    (@array [$($elems:expr),*] $unexpected:tt $($rest:tt)*) => {
        $crate::value_unexpected!($unexpected)
    };

    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: value!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////////

    // Done.
    (@object $object:ident () () ()) => {};

    (@object $object:ident [$($key:ident)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(String::from(stringify!($($key)+)), $value);
        $crate::value!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        let _ = $object.insert(($($key)+).into(), $value);
        $crate::value!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Current entry followed by unexpected token.
    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        $crate::value_unexpected!($unexpected);
    };

    (@object $object:ident [$($key:ident)+] ($value:expr)) => {
        let _ = $object.insert(($(String::from(stringify!($key)))+).into(), $value);
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        let _ = $object.insert(($($key)+).into(), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!(null)) $($rest)*);
    };

    // Next value is `true`.
    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!(true)) $($rest)*);
    };

    // Next value is `false`.
    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!(false)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::value!(@object $object [$($key)+] ($crate::value!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::value!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::value!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        $crate::value_unexpected!($colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        $crate::value_unexpected!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Refuse to absorb colon token into key expression.
    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        $crate::value_expect_expr_comma!($($unexpected)+);
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::value!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: value!($($value)+)
    //////////////////////////////////////////////////////////////////////////

    (null) => {
        $crate::Value::Null
    };

    (true) => {
        $crate::Value::Boolean(true)
    };

    (false) => {
        $crate::Value::Boolean(false)
    };

    ([]) => {
        $crate::Value::Array(vec![])
    };

    ([ $($tt:tt)+ ]) => {
        $crate::Value::Array($crate::value!(@array [] $($tt)+))
    };

    ({}) => {
        $crate::Value::Object(::std::collections::BTreeMap::new())
    };

    ({ $($tt:tt)+ }) => {
        $crate::Value::Object({
            let mut object = ::std::collections::BTreeMap::new();
            $crate::value!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };

    // Any Serialize type: numbers, strings, struct literals, variables etc.
    // Must be below every other rule.
    ($other:expr) => {
        $crate::Value::from($other)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! value_unexpected {
    () => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! value_expect_expr_comma {
    ($e:expr , $($tt:tt)*) => {};
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
