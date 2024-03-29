use std::time::Duration;

pub const fn default_true() -> bool {
    true
}

pub const fn default_interval() -> Duration {
    Duration::from_secs(15)
}

/// Answers "Is it possible to skip serializing this value, because it's the
/// default?"
#[inline]
pub fn skip_serializing_if_default<T: Default + PartialEq>(e: &T) -> bool {
    e == &T::default()
}

pub mod serde_regex {
    use std::borrow::Cow;

    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<regex::Regex, D::Error> {
        let s: Cow<str> = serde::Deserialize::deserialize(deserializer)?;
        regex::Regex::new(&s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(re: &regex::Regex, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(re.as_str())
    }
}

pub mod serde_uri {
    use std::borrow::Cow;

    use http::Uri;
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Uri, D::Error> {
        let s: Cow<str> = serde::Deserialize::deserialize(deserializer)?;
        s.parse::<Uri>().map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(uri: &Uri, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&uri.to_string())
    }
}

pub mod serde_http_method {
    use http::Method;
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Method, D::Error> {
        let s: &str = serde::Deserialize::deserialize(deserializer)?;
        Method::try_from(s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(method: &Method, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(method.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct RE {
        #[serde(with = "serde_regex")]
        re: regex::Regex,
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
