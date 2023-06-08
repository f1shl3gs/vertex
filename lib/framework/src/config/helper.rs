mod charset;

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
pub fn skip_serializing_if_default<E: Default + PartialEq>(e: &E) -> bool {
    e == &E::default()
}

pub const fn default_acknowledgements() -> bool {
    false
}

pub mod serde_regex {
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<regex::Regex, D::Error> {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        regex::Regex::new(&s).map_err(serde::de::Error::custom)
    }

    pub fn serialize<S: Serializer>(re: &regex::Regex, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(re.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct RE {
        #[serde(with = "serde_regex")]
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
