use std::fmt::{Debug, Display, Formatter, Write};
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{BorrowedSegment, PathParseError, PathPrefix, ValuePath};
use super::{parse_target_path, parse_value_path};

/// A lookup path.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OwnedValuePath {
    pub segments: Vec<OwnedSegment>,
}

impl OwnedValuePath {
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn root() -> Self {
        vec![].into()
    }

    pub fn push_field(&mut self, field: &str) {
        self.segments.push(OwnedSegment::field(field));
    }

    pub fn push_segment(&mut self, segment: OwnedSegment) {
        self.segments.push(segment);
    }

    pub fn push_front_field(&mut self, field: &str) {
        self.segments.insert(0, OwnedSegment::field(field));
    }

    pub fn push_front_segment(&mut self, segment: OwnedSegment) {
        self.segments.insert(0, segment);
    }

    pub fn with_field_appended(&self, field: &str) -> Self {
        let mut new_path = self.clone();
        new_path.push_field(field);
        new_path
    }

    pub fn with_field_prefix(&self, field: &str) -> Self {
        self.with_segment_prefix(OwnedSegment::field(field))
    }

    pub fn with_segment_prefix(&self, segment: OwnedSegment) -> Self {
        let mut new_path = self.clone();
        new_path.push_front_segment(segment);
        new_path
    }

    pub fn push_index(&mut self, index: isize) {
        self.segments.push(OwnedSegment::index(index));
    }

    pub fn with_index_appended(&self, index: isize) -> Self {
        let mut new_path = self.clone();
        new_path.push_index(index);
        new_path
    }

    pub fn single_field(field: &str) -> Self {
        vec![OwnedSegment::field(field)].into()
    }

    /// Create the possible fields that can be followed by this lookup.
    /// Because of coalesced paths there can be a number of different combinations.
    /// There is the potential for this function to create a vast number of different
    /// combinations if there are multiple coalesced segments in a path.
    ///
    /// The limit specifies the limit of the path depth we are interested in.
    /// Metrics is only interested in fields that are up to 3 levels deep (2 levels + 1 to check it
    /// terminates).
    ///
    /// eg, .tags.nork.noog will never be an accepted path so we don't need to spend the time
    /// collecting it.
    pub fn to_alternative_components(&self, limit: usize) -> Vec<Vec<&str>> {
        let mut components = vec![vec![]];
        for segment in self.segments.iter().take(limit) {
            match segment {
                OwnedSegment::Field(field) => {
                    for component in &mut components {
                        component.push(field.as_str());
                    }
                }
                OwnedSegment::Index(_) => {
                    return Vec::new();
                }
            }
        }

        components
    }

    pub fn push(&mut self, segment: OwnedSegment) {
        self.segments.push(segment);
    }
}

// OwnedValuePath values must have at least one segment.
#[cfg(test)]
impl proptest::prelude::Arbitrary for OwnedValuePath {
    type Parameters = ();
    type Strategy = proptest::prelude::BoxedStrategy<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        prop::collection::vec(any::<OwnedSegment>(), 1..10)
            .prop_map(|segments| OwnedValuePath { segments })
            .boxed()
    }
}

impl Display for OwnedValuePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl FromStr for OwnedValuePath {
    type Err = PathParseError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        parse_value_path(src).map_err(|_| PathParseError::InvalidPathSyntax {
            path: src.to_owned(),
        })
    }
}

impl TryFrom<String> for OwnedValuePath {
    type Error = PathParseError;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        parse_value_path(&src).map_err(|_| PathParseError::InvalidPathSyntax {
            path: src.to_owned(),
        })
    }
}

impl From<OwnedValuePath> for String {
    fn from(owned: OwnedValuePath) -> Self {
        Self::from(&owned)
    }
}

impl From<&OwnedValuePath> for String {
    fn from(owned: &OwnedValuePath) -> Self {
        let mut output = String::new();

        for (i, segment) in owned.segments.iter().enumerate() {
            match segment {
                OwnedSegment::Field(field) => {
                    serialize_field(&mut output, field.as_ref(), (i != 0).then_some("."))
                }
                OwnedSegment::Index(index) => {
                    write!(output, "[{index}]").expect("Could not write to string")
                }
            }
        }

        output
    }
}

impl From<Vec<OwnedSegment>> for OwnedValuePath {
    fn from(segments: Vec<OwnedSegment>) -> Self {
        Self { segments }
    }
}

impl Serialize for OwnedValuePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

impl<'de> Deserialize<'de> for OwnedValuePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PathVisitor;
        impl serde::de::Visitor<'_> for PathVisitor {
            type Value = OwnedValuePath;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string is expected")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                parse_value_path(v).map_err(|_err| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"valid event ValuePath",
                    )
                })
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v)
            }
        }

        deserializer.deserialize_str(PathVisitor)
    }
}

/// An owned path that contains a target (pointing to either an Event or Metadata)
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct OwnedTargetPath {
    pub prefix: PathPrefix,
    pub path: OwnedValuePath,
}

impl OwnedTargetPath {
    pub fn event_root() -> Self {
        Self::root(PathPrefix::Event)
    }
    pub fn metadata_root() -> Self {
        Self::root(PathPrefix::Metadata)
    }

    pub fn root(prefix: PathPrefix) -> Self {
        Self {
            prefix,
            path: OwnedValuePath::root(),
        }
    }

    pub fn event(path: OwnedValuePath) -> Self {
        Self {
            prefix: PathPrefix::Event,
            path,
        }
    }

    pub fn metadata(path: OwnedValuePath) -> Self {
        Self {
            prefix: PathPrefix::Metadata,
            path,
        }
    }

    pub fn can_start_with(&self, prefix: &Self) -> bool {
        if self.prefix != prefix.prefix {
            return false;
        }
        (&self.path).can_start_with(&prefix.path)
    }

    pub fn with_field_appended(&self, field: &str) -> Self {
        let mut new_path = self.path.clone();
        new_path.push_field(field);
        Self {
            prefix: self.prefix,
            path: new_path,
        }
    }

    pub fn with_index_appended(&self, index: isize) -> Self {
        let mut new_path = self.path.clone();
        new_path.push_index(index);
        Self {
            prefix: self.prefix,
            path: new_path,
        }
    }
}

impl Display for OwnedTargetPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self.to_owned()))
    }
}

impl Debug for OwnedTargetPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl From<OwnedTargetPath> for String {
    fn from(target_path: OwnedTargetPath) -> Self {
        match target_path.prefix {
            PathPrefix::Event => format!(".{}", target_path.path),
            PathPrefix::Metadata => format!("%{}", target_path.path),
        }
    }
}

impl FromStr for OwnedTargetPath {
    type Err = PathParseError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        parse_target_path(src).map_err(|_| PathParseError::InvalidPathSyntax {
            path: src.to_owned(),
        })
    }
}

impl TryFrom<String> for OwnedTargetPath {
    type Error = PathParseError;

    fn try_from(src: String) -> Result<Self, Self::Error> {
        parse_target_path(&src).map_err(|_| PathParseError::InvalidPathSyntax {
            path: src.to_owned(),
        })
    }
}

fn serialize_field(string: &mut String, field: &str, separator: Option<&str>) {
    // These characters should match the ones from the parser, implemented in `JitLookup`
    let needs_quotes = field.is_empty()
        || field
            .chars()
            .any(|c| !matches!(c, 'A'..='Z' | 'a'..='z' | '_' | '0'..='9' | '@'));

    // Allocate enough to fit the field, a `.` and two `"` characters. This
    // should suffice for the majority of cases when no escape sequence is used.
    let separator_len = separator.map(|x| x.len()).unwrap_or(0);
    string.reserve(field.len() + 2 + separator_len);
    if let Some(separator) = separator {
        string.push_str(separator);
    }
    if needs_quotes {
        string.push('"');
        for c in field.chars() {
            if matches!(c, '"' | '\\') {
                string.push('\\');
            }
            string.push(c);
        }
        string.push('"');
    } else {
        string.push_str(field);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum OwnedSegment {
    Field(String),
    Index(isize),
}

impl OwnedSegment {
    pub fn field(value: &str) -> OwnedSegment {
        OwnedSegment::Field(value.to_string())
    }
    pub fn index(value: isize) -> OwnedSegment {
        OwnedSegment::Index(value)
    }

    pub fn is_field(&self) -> bool {
        matches!(self, OwnedSegment::Field(_))
    }
    pub fn is_index(&self) -> bool {
        matches!(self, OwnedSegment::Index(_))
    }

    pub fn can_start_with(&self, prefix: &OwnedSegment) -> bool {
        match (self, prefix) {
            (OwnedSegment::Index(a), OwnedSegment::Index(b)) => a == b,
            (OwnedSegment::Index(_), _) | (_, OwnedSegment::Index(_)) => false,
            (OwnedSegment::Field(a), OwnedSegment::Field(b)) => a == b,
        }
    }
}

impl<'a> From<&'a str> for OwnedSegment {
    fn from(field: &'a str) -> Self {
        OwnedSegment::field(field)
    }
}

impl<'a> From<&'a String> for OwnedSegment {
    fn from(field: &'a String) -> Self {
        OwnedSegment::field(field.as_str())
    }
}

impl From<isize> for OwnedSegment {
    fn from(index: isize) -> Self {
        OwnedSegment::index(index)
    }
}

impl<'a> TryFrom<BorrowedSegment<'a>> for OwnedSegment {
    type Error = ();

    fn try_from(segment: BorrowedSegment<'a>) -> Result<Self, Self::Error> {
        match segment {
            BorrowedSegment::Field(field) => Ok(OwnedSegment::Field(field.into())),
            BorrowedSegment::Index(index) => Ok(OwnedSegment::Index(index)),
            BorrowedSegment::Invalid => Err(()),
        }
    }
}

impl Display for OwnedSegment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnedSegment::Index(i) => write!(f, "[{i}]"),
            OwnedSegment::Field(field) => f.write_str(field),
        }
    }
}

impl<'a> ValuePath<'a> for &'a Vec<OwnedSegment> {
    type Iter = OwnedSegmentSliceIter<'a>;

    fn segment_iter(&self) -> Self::Iter {
        OwnedSegmentSliceIter(self.iter())
    }
}

impl<'a> ValuePath<'a> for &'a [OwnedSegment] {
    type Iter = OwnedSegmentSliceIter<'a>;

    fn segment_iter(&self) -> Self::Iter {
        OwnedSegmentSliceIter(self.iter())
    }
}

impl<'a> ValuePath<'a> for &'a OwnedValuePath {
    type Iter = OwnedSegmentSliceIter<'a>;

    fn segment_iter(&self) -> Self::Iter {
        (&self.segments).segment_iter()
    }
}

#[derive(Clone)]
pub struct OwnedSegmentSliceIter<'a>(std::slice::Iter<'a, OwnedSegment>);

impl<'a> Iterator for OwnedSegmentSliceIter<'a> {
    type Item = BorrowedSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(BorrowedSegment::from)
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use crate::path::parse_value_path;

    #[test]
    fn owned_path_serialize() {
        let test_cases = [
            (".", Some("")),
            ("", None),
            ("]", None),
            ("]foo", None),
            ("..", None),
            ("...", None),
            ("f", Some("f")),
            ("foo", Some("foo")),
            (
                r#"ec2.metadata."availability-zone""#,
                Some(r#"ec2.metadata."availability-zone""#),
            ),
            ("@timestamp", Some("@timestamp")),
            ("foo[", None),
            ("foo$", None),
            (r#""$peci@l chars""#, Some(r#""$peci@l chars""#)),
            ("foo.foo bar", None),
            (r#"foo."foo bar".bar"#, Some(r#"foo."foo bar".bar"#)),
            ("[1]", Some("[1]")),
            ("[42]", Some("[42]")),
            ("foo.[42]", None),
            ("[42].foo", Some("[42].foo")),
            ("[-1]", Some("[-1]")),
            ("[-42]", Some("[-42]")),
            ("[-42].foo", Some("[-42].foo")),
            ("[-42]foo", Some("[-42].foo")),
            (r#""[42]. {}-_""#, Some(r#""[42]. {}-_""#)),
            (r#""a\"a""#, Some(r#""a\"a""#)),
            (r#"foo."a\"a"."b\\b".bar"#, Some(r#"foo."a\"a"."b\\b".bar"#)),
            ("<invalid>", None),
            (r#""🤖""#, Some(r#""🤖""#)),
        ];

        for (path, expected) in test_cases {
            let path = parse_value_path(path).map(String::from).ok();

            assert_eq!(path, expected.map(|x| x.to_owned()));
        }
    }

    fn reparse_thing<T: Debug + Display + Eq + FromStr>(thing: T)
    where
        <T as FromStr>::Err: Debug,
    {
        let text = thing.to_string();
        let thing2: T = text.parse().unwrap();
        assert_eq!(thing, thing2);
    }

    proptest::proptest! {
        #[test]
        fn reparses_valid_value_path(path: OwnedValuePath) {
            reparse_thing(path);
        }

        #[test]
        fn reparses_valid_target_path(path: OwnedTargetPath) {
            reparse_thing(path);
        }
    }
}
