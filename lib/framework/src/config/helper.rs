mod charset;

use std::time::Duration;

pub use serde_regex::*;

pub const fn default_true() -> bool {
    true
}

pub const fn default_false() -> bool {
    false
}

pub const fn default_interval() -> std::time::Duration {
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
