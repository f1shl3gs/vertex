use std::borrow::Cow;
use serde::{Deserializer, Serializer};
use regex::Regex;

use crate::duration::parse_duration;

pub fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<chrono::Duration, D::Error> {
    let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_duration<S: Serializer>(_d: &chrono::Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("1m")
}

pub fn deserialize_regex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Regex::new(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_regex<S: Serializer>(re: &Regex, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct RE {
        #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
        re: Regex
    }

    #[test]
    fn test_regex_serde() {
        let re: RE = serde_yaml::from_str(r#"
        re: .*
        "#).unwrap();

        println!("{}", re.re.as_str())
    }
}