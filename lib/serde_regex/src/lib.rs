use serde::{Deserializer, Serializer};

pub fn deserialize_regex<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<::regex::Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    regex::Regex::new(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_regex<S: Serializer>(re: &::regex::Regex, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}

pub fn deserialize_bytes_regex<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<regex::bytes::Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    regex::bytes::Regex::new(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_bytes_regex<S: Serializer>(
    re: &regex::bytes::Regex,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}
