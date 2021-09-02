use serde::{Deserialize, Serialize};
use regex::Regex;
use crate::{
    config::{deserialize_regex, serialize_regex}
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Config {
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    Include(Regex),

    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    Exclude(Regex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let conf :Config = serde_yaml::from_str(r#"
include: .*
        "#).unwrap();
    }
}