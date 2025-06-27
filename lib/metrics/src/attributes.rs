use std::borrow::Cow;
use std::collections::BTreeMap;

/// A set of key-value pairs with unique keys
///
/// A `Metric` records observations for each unique set of `Attributes`
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Attributes(BTreeMap<&'static str, Cow<'static, str>>);

impl Attributes {
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, &'static str, Cow<'static, str>> {
        self.0.iter()
    }

    /// Sets the given key, overriding it if already set
    pub fn insert(&mut self, key: &'static str, value: impl Into<Cow<'static, str>>) {
        self.0.insert(key, value.into());
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

impl<'a, const N: usize> From<&'a [(&'static str, &'static str); N]> for Attributes {
    fn from(iterator: &'a [(&'static str, &'static str); N]) -> Self {
        Self(
            iterator
                .iter()
                .map(|(key, value)| {
                    assert_legal_key(key);
                    (*key, Cow::Borrowed(*value))
                })
                .collect(),
        )
    }
}

impl<const N: usize> From<[(&'static str, Cow<'static, str>); N]> for Attributes {
    fn from(iterator: [(&'static str, Cow<'static, str>); N]) -> Self {
        Self(
            IntoIterator::into_iter(iterator)
                .map(|(key, value)| {
                    assert_legal_key(key);
                    (key, value)
                })
                .collect(),
        )
    }
}

/// Panics if the provided string matches [0-9a-z_]+
pub fn assert_legal_key(s: &str) {
    assert!(!s.is_empty(), "string must not be empty");
    assert!(
        s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='z' | '_')),
        "string must be [0-9a-z_]+ got: \"{s}\""
    )
}
