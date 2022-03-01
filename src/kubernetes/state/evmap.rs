use super::hash_value::HashValue;

/// An alias to the value used at [`evmap`]
pub type Value<T> = Box<HashValue<T>>;