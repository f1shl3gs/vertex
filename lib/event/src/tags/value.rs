use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

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
                for v in f {
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
