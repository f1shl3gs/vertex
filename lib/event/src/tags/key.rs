use std::borrow::Cow;
use std::fmt;
use std::ops::Deref;

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

/// Key used for `Metric`s and trace `Span` attributes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Key(Cow<'static, str>);

impl Key {
    /// Create a new `Key`.
    pub fn new<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Key(value.into())
    }

    /// Create a new const `Key`.
    #[inline]
    pub const fn from_static(value: &'static str) -> Self {
        Key(Cow::Borrowed(value))
    }

    /// Returns a reference to the underlying key name
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl AsRef<str> for Key {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Deref for Key {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl ByteSizeOf for Key {
    fn allocated_bytes(&self) -> usize {
        match &self.0 {
            Cow::Borrowed(_) => 0,
            Cow::Owned(s) => s.len(),
        }
    }
}

impl From<&'static str> for Key {
    /// Convert a `&str` to a `Key`.
    fn from(s: &'static str) -> Self {
        Key(Cow::Borrowed(s))
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
        Key(Cow::Owned(string))
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
