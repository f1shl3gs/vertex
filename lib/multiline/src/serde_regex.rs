use serde::{Deserializer, Serializer};

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<regex::bytes::Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    regex::bytes::Regex::new(&s).map_err(serde::de::Error::custom)
}

pub(super) fn serialize<S: Serializer>(re: &regex::bytes::Regex, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}
