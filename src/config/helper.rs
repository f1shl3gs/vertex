mod charset;

use humanize::{duration_to_string, parse_duration};
use serde::{Deserialize, Deserializer, Serializer};
use std::borrow::Cow;
use tokio_stream::wrappers::IntervalStream;

pub fn deserialize_duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
    parse_duration(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_duration<S: Serializer>(d: &chrono::Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&duration_to_string(d))
}

pub fn deserialize_duration_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<chrono::Duration>, D::Error> {
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(text) => {
            let duration = parse_duration(&text).map_err(serde::de::Error::custom)?;
            Ok(Some(duration))
        }
        None => Ok(None),
    }
}

pub fn serialize_duration_option<S: Serializer>(
    d: &Option<chrono::Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match d {
        Some(d) => s.serialize_str(&duration_to_string(d)),
        None => s.serialize_none(),
    }
}

pub fn deserialize_std_duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<std::time::Duration, D::Error> {
    let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
    humanize::duration_std::parse_duration(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_std_duration<S: Serializer>(
    d: &std::time::Duration,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.serialize_str(&humanize::duration_std::duration_to_string(d))
}

pub fn deserialize_std_duration_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<std::time::Duration>, D::Error> {
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(text) => {
            let duration =
                humanize::duration_std::parse_duration(&text).map_err(serde::de::Error::custom)?;
            Ok(Some(duration))
        }
        None => Ok(None),
    }
}

pub fn serialize_std_duration_option<S: Serializer>(
    d: &Option<std::time::Duration>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match d {
        Some(d) => s.serialize_str(&humanize::duration_std::duration_to_string(d)),
        None => s.serialize_none(),
    }
}

pub fn deserialize_regex<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<::regex::Regex, D::Error> {
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    ::regex::Regex::new(&s).map_err(serde::de::Error::custom)
}

pub fn serialize_regex<S: Serializer>(re: &::regex::Regex, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(re.as_str())
}

pub const fn default_true() -> bool {
    true
}

pub const fn default_false() -> bool {
    false
}

pub fn default_interval() -> chrono::Duration {
    chrono::Duration::seconds(15)
}

pub fn default_std_interval() -> std::time::Duration {
    std::time::Duration::from_secs(15)
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
    let duration = duration.to_std().map_err(|_| ())?;
    let interval = tokio::time::interval(duration.into());
    Ok(IntervalStream::new(interval))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct RE {
        #[serde(
            deserialize_with = "deserialize_regex",
            serialize_with = "serialize_regex"
        )]
        re: ::regex::Regex,
    }

    #[test]
    fn test_regex_serde() {
        let re: RE = serde_yaml::from_str(
            r#"
        re: .*
        "#,
        )
        .unwrap();

        assert_eq!(re.re.as_str(), ".*");
    }
}
