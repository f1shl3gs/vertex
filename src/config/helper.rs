use std::borrow::Cow;
use serde::{Deserialize, Deserializer, Serializer};
use tokio_stream::wrappers::IntervalStream;

use crate::duration::{duration_to_string, parse_duration};

pub mod duration {
    use std::borrow::Cow;
    use serde::{Deserializer, Serializer};
    use crate::duration::{duration_to_string, parse_duration};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<chrono::Duration, D::Error> {
        let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
        parse_duration(&s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(_d: &chrono::Duration, s: S) -> Result<S::Ok, S::Error> {
        let d = duration_to_string(_d);
        s.serialize_str(&d)
    }
}

#[deprecated]
pub fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<chrono::Duration, D::Error> {
    let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}

#[deprecated]
pub fn serialize_duration<S: Serializer>(_d: &chrono::Duration, s: S) -> Result<S::Ok, S::Error> {
    let d = duration_to_string(_d);
    s.serialize_str(&d)
}

pub fn deserialize_duration_option<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<chrono::Duration>, D::Error> {
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(text) => {
            let duration = parse_duration(&text).map_err(serde::de::Error::custom)?;
            Ok(Some(duration))
        }
        None => Ok(None)
    }
}

pub fn serialize_duration_option<S: Serializer>(d: &Option<chrono::Duration>, s: S) -> Result<S::Ok, S::Error> {
    match d {
        Some(d) => s.serialize_str(&duration_to_string(d)),
        None => s.serialize_none()
    }
}

pub mod regex {
    use serde::{Deserializer, Serializer};
    use regex::Regex;

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Regex, D::Error> {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        Regex::new(&s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(re: &Regex, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(re.as_str())
    }
}

#[deprecated]
pub fn deserialize_regex<'de, D: Deserializer<'de>>(deserializer: D) -> Result<::regex::Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    ::regex::Regex::new(&s).map_err(serde::de::Error::custom)
}

#[deprecated]
pub fn serialize_regex<S: Serializer>(re: &::regex::Regex, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}

pub fn default_true() -> bool {
    true
}

pub fn default_false() -> bool {
    false
}

pub fn default_interval() -> chrono::Duration {
    chrono::Duration::seconds(15)
}

/// Answers "Is it possible to skip serializing this value, because it's the
/// default?"
#[inline]
pub fn skip_serializing_if_default<E: Default + PartialEq>(e: &E) -> bool {
    e == &E::default()
}

pub const fn default_acknowledgements() -> bool {
    false
}

pub fn ticker_from_duration(duration: chrono::Duration) -> Result<IntervalStream, ()> {
    let duration = duration.to_std()
        .map_err(|_| ())?;
    let interval = tokio::time::interval(duration.into());
    Ok(IntervalStream::new(interval))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct RE {
        #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
        re: ::regex::Regex,
    }

    #[test]
    fn test_regex_serde() {
        let re: RE = serde_yaml::from_str(r#"
        re: .*
        "#).unwrap();

        println!("{}", re.re.as_str())
    }
}