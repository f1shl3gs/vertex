use std::fmt::{Formatter, Write};
use std::time::Duration;

use regex::bytes::Regex;
use serde::de::{Error, MapAccess};
use serde::{de, Deserialize, Deserializer, Serialize};
use snafu::{ResultExt, Snafu};

use super::aggregate::{self, Mode};

/*
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display(
        "unable to parse multiline start pattern from {:?}: {}",
        start_pattern,
        source
    ))]
    InvalidMultilineStartPattern {
        start_pattern: String,
        source: regex::Error,
    },
    #[snafu(display(
        "unable to parse multiline condition pattern from {:?}: {}",
        condition_pattern,
        source
    ))]
    InvalidMultilineConditionPattern {
        condition_pattern: String,
        source: regex::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MultilineConfig {
    pub start_pattern: String,
    pub condition_pattern: String,
    pub timeout: Duration,
    pub mode: Mode,
}

impl TryFrom<&MultilineConfig> for aggregate::Config {
    type Error = Error;

    fn try_from(config: &MultilineConfig) -> Result<Self, Self::Error> {
        let MultilineConfig {
            start_pattern,
            condition_pattern,
            mode,
            timeout,
        } = config;

        let start_pattern = Regex::new(start_pattern)
            .with_context(|| InvalidMultilineStartPattern { start_pattern })?;
        let condition_pattern = Regex::new(condition_pattern)
            .with_context(|| InvalidMultilineConditionPattern { condition_pattern })?;
        let timeout = *timeout;

        Ok(Self {
            start_pattern,
            condition_pattern,
            mode: *mode,
            timeout,
        })
    }
}
*/

#[derive(Debug)]
struct DockerRule {}


///
/// With type
/// ```yaml
/// multiline:
///     type: java
///     timeout: 5s # this should be default
/// ```
///
/// Without type
/// ```yaml
/// multiline:
///     type: custom
///     timeout: 5s
///     start_pattern:  ^[^\s]
///     condition_pattern: ^[\s]+
///     mode: continue_through
/// ```
#[derive(Debug)]
pub enum MultilineConfig {
    Cri,
    Docker,
    Go,
    Java,

    Custom,
}

impl<'de> serde::de::Deserialize<'de> for MultilineConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrMap;

        impl<'de> serde::de::Visitor<'de> for StringOrMap {
            type Value = MultilineConfig;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "docker" => Ok(MultilineConfig::Docker),
                    "cri" => Ok(MultilineConfig::Cri),
                    "go" => Ok(MultilineConfig::Go),
                    "java" => Ok(MultilineConfig::Java),
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(v),
                        &r#"cri, docker, go or java"#,
                    )),
                }
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                todo!()
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}
