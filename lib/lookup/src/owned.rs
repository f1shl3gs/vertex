use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct OwnedPath {
    pub segments: Vec<OwnedSegment>,
}

impl OwnedPath {
    pub fn root() -> Self {
        vec![].into()
    }

    pub fn push_field(&mut self, field: &str) {
        self.segments.push(OwnedSegment::field(field))
    }

    pub fn with_field_appended(&self, field: &str) -> Self {
        let mut new_path = self.clone();
        new_path.push_field(field);
        new_path
    }

    pub fn push_index(&mut self, index: isize) {
        self.segments.push(OwnedSegment::index(index))
    }

    pub fn with_index_appended(&self, index: isize) -> Self {
        let mut new_path = self.clone();
        new_path.push_index(index);
        new_path
    }

    pub fn single_field(field: &str) -> Self {
        vec![OwnedSegment::field(field)].into()
    }
}

impl<'de> Deserialize<'de> for OwnedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let path: String = Deserialize::deserialize(deserializer)?;
        Ok(parse_path(&path))
    }
}

impl Serialize for OwnedPath {
    fn serialize<S>(&self, serializer: S) -> Result<serde::ser::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.segments.is_empty() {
            return serializer.serialize_str("<invalid>");
        }

        let mut coalesce_i = 0;
        let path = self
            .segments
            .iter()
            .enumerate()
            .map(|(i, segment)| match segment {
                OwnedSegment::Field(field) => {
                    serialize_field(field.as_ref(), (i != 0).then(|| "."))
                }

                OwnedSegment::CoalesceField(field) => {
                    let output = serialize_field(
                        field.as_ref(),
                        Some(if coalesce_i == 0 {
                            if i == 0 {
                                "("
                            } else {
                                ".("
                            }
                        } else {
                            "|"
                        }),
                    );

                    coalesce_i += 1;
                    output
                }
                OwnedSegment::Index(index) => format!("[{}]", index),
                OwnedSegment::Invalid => {
                    (if i == 0 { "<invalid>" } else { ".<invalid>" }).to_owned()
                }
                OwnedSegment::CoalesceEnd(field) => {
                    format!(
                        "{})",
                        serialize_field(field.as_ref(), (coalesce_i != 0).then(|| "|"))
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("");

        serializer.serialize_str(&path)
    }
}

fn serialize_field(field: &str, separator: Option<&str>) -> String {
    // These characters should match the ones from the parser, implemented in `JitLookup`
    let needs_quotes = field
        .chars()
        .any(|c| !matches!(c, 'A'..'Z' | 'a'..'z' | '_' | '0'..='9' | '@'));

    // Allocate enough to fit the field, a `.` and two `"` characters. This should
    // suffice for the majority of cases when no escape sequence is used.
    let separator_len = separator.map(|x| x.len()).unwrap_or(0);
    let mut s = String::with_capacity(field.as_bytes().len() + 2 + separator_len);
    if let Some(separator) = separator {
        s.push_str(separator);
    }

    if needs_quotes {
        s.push('"');

        for c in field.chars() {
            if matches!(c, '"' | '\\') {
                s.push('\\');
            }

            s.push(c);
        }
        s.push('"')
    } else {
        s.push_str(field);
    }

    s
}

impl From<Vec<OwnedSegment>> for OwnedPath {
    fn from(segments: Vec<OwnedSegment>) -> Self {
        Self { segments }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedSegment {
    Field(String),
    Index(isize),
    CoalesceField(String),
    CoalesceEnd(String),
    Invalid,
}

impl OwnedSegment {
    pub fn field(value: &str) -> OwnedSegment {
        OwnedSegment::Field(value.to_string())
    }

    pub fn index(value: isize) -> OwnedSegment {
        OwnedSegment::Index(value)
    }

    pub fn coalesce_field(field: impl Into<String>) -> OwnedSegment {
        OwnedSegment::CoalesceField(field.into())
    }

    pub fn coalesce_end(field: impl Into<String>) -> OwnedSegment {
        OwnedSegment::CoalesceEnd(field.into())
    }

    pub fn is_field(&self) -> bool {
        matches!(self, OwnedSegment::Field(_))
    }

    pub fn is_index(&self) -> bool {
        matches!(self, OwnedSegment::Index(_))
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, OwnedSegment::Invalid)
    }
}

impl<'a> Path<'a> for &'a OwnedPath {

}