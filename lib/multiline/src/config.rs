use std::fmt::Formatter;
use std::time::Duration;

use humanize::duration::parse_duration;
use regex::bytes::Regex;
use serde::de::{Error, MapAccess, Unexpected};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::aggregate::Mode;

const CRI_PARSER: &str = "cri";
const DOCKER_PARSER: &str = "docker";
const GO_PARSER: &str = "go";
const JAVA_PARSER: &str = "java";
const NO_INDENT: &str = "no_indent";
const CUSTOM_PARSER: &str = "custom";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Parser {
    #[default]
    NoIndent,
    Cri,
    Docker,
    Go,
    Java,

    Custom {
        #[serde(with = "crate::serde_regex")]
        condition_pattern: Regex,

        #[serde(with = "crate::serde_regex")]
        start_pattern: Regex,

        mode: Mode,
    },
}

impl PartialEq for Parser {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Cri => matches!(other, Parser::Cri),
            Self::Docker => matches!(other, Parser::Docker),
            Self::Go => matches!(other, Parser::Go),
            Self::Java => matches!(other, Parser::Java),
            Self::NoIndent => matches!(other, Parser::NoIndent),
            Self::Custom {
                mode,
                condition_pattern,
                start_pattern,
            } => {
                let m1 = mode;
                let c1 = condition_pattern;
                let s1 = start_pattern;

                match other {
                    Parser::Custom {
                        mode,
                        condition_pattern,
                        start_pattern,
                    } => {
                        if mode != m1 {
                            return false;
                        }

                        if condition_pattern.as_str() != c1.as_str() {
                            return false;
                        }

                        if start_pattern.as_str() != s1.as_str() {
                            return false;
                        }

                        true
                    }

                    _ => unreachable!(),
                }
            }
        }
    }
}

/// Preset parser with default timeout
/// ```yaml
/// multiline: cri
/// # equal to
/// # multiline:
/// #   parser: cri
/// ```
///
/// Preset parser with custom timeout
/// ```yaml
/// multiline:
///     type: java
///     timeout: 2s
/// ```
///
/// Custom type, except `timeout` all field is required
/// ```yaml
/// multiline:
///     timeout: 5s
///     parser: custom
///     start_pattern:  ^[^\s]
///     condition_pattern: ^[\s]+
///     mode: continue_through
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The maximum amount of time to wait for the next additional line.
    ///
    /// Once this timeout is reached, the buffered message is guaranteed to be flushed,
    /// even if incomplete.
    pub timeout: Duration,

    pub parser: Parser,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrMap;

        impl<'de> serde::de::Visitor<'de> for StringOrMap {
            type Value = Config;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("string or map")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                match v {
                    "cri" => Ok(Config {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Cri,
                    }),
                    "docker" => Ok(Config {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Docker,
                    }),
                    "go" => Ok(Config {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Go,
                    }),
                    "java" => Ok(Config {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Java,
                    }),
                    "no_indent" => Ok(Config {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::NoIndent,
                    }),
                    _ => Err(Error::invalid_value(
                        Unexpected::Str(v),
                        &r#"cri, docker, go, java, no_ident or custom"#,
                    )),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut timeout = None;
                let mut start_pattern = None;
                let mut condition_pattern = None;
                let mut mode = None;
                let mut parser = None;

                // serde_json can work with `&str` or `String`, while serde_yaml cannot work with `&str`,
                // so the string is allocated here.
                while let Some((key, value)) = map.next_entry::<&str, String>()? {
                    match key {
                        "timeout" => {
                            if timeout.is_some() {
                                return Err(Error::duplicate_field("timeout"));
                            }

                            let v = parse_duration(&value).map_err(|_err| {
                                Error::invalid_value(
                                    Unexpected::Str(&value),
                                    &r#"something like 5s, 10s"#,
                                )
                            })?;

                            timeout = Some(v);
                        }
                        "start_pattern" => {
                            if start_pattern.is_some() {
                                return Err(Error::duplicate_field("start_pattern"));
                            }

                            let re = Regex::new(&value).map_err(|_err| {
                                Error::invalid_value(
                                    Unexpected::Str(&value),
                                    &r#"regex is expected"#,
                                )
                            })?;

                            start_pattern = Some(re);
                        }

                        "condition_pattern" => {
                            if condition_pattern.is_some() {
                                return Err(Error::duplicate_field("condition_pattern"));
                            }

                            let re = Regex::new(&value).map_err(|_err| {
                                Error::invalid_value(
                                    Unexpected::Str(&value),
                                    &r#"regex is ecpected"#,
                                )
                            })?;

                            condition_pattern = Some(re);
                        }

                        "parser" => {
                            if parser.is_some() {
                                return Err(Error::duplicate_field("parser"));
                            }

                            parser = match value.as_str() {
                                CRI_PARSER => Some(CRI_PARSER),
                                DOCKER_PARSER => Some(DOCKER_PARSER),
                                GO_PARSER => Some(GO_PARSER),
                                JAVA_PARSER => Some(JAVA_PARSER),
                                NO_INDENT => Some(NO_INDENT),
                                CUSTOM_PARSER => Some(CUSTOM_PARSER),
                                _ => {
                                    return Err(Error::unknown_variant(
                                        "parser",
                                        &["cri", "docker", "go", "java", "no_indent", "custom"],
                                    ));
                                }
                            }
                        }

                        "mode" => {
                            if mode.is_some() {
                                return Err(Error::duplicate_field("mode"));
                            }

                            mode = Some(match value.as_str() {
                                "continue_through" => Mode::ContinueThrough,
                                "continue_past" => Mode::ContinuePast,
                                "halt_before" => Mode::HaltBefore,
                                "halt_with" => Mode::HaltWith,
                                _ => {
                                    return Err(Error::unknown_variant(
                                        "mode",
                                        &[
                                            "continue_through",
                                            "continue_past",
                                            "halt_before",
                                            "halt_with",
                                        ],
                                    ));
                                }
                            });
                        }

                        _ => {
                            return Err(Error::unknown_field(
                                key,
                                &[
                                    "parser",
                                    "timeout",
                                    "start_pattern",
                                    "condition_pattern",
                                    "mode",
                                ],
                            ));
                        }
                    }
                }

                let timeout = timeout.unwrap_or(DEFAULT_TIMEOUT);

                match parser {
                    None => Err(Error::missing_field("parser")),

                    Some(CUSTOM_PARSER) => {
                        if condition_pattern.is_none() {
                            return Err(Error::missing_field("condition_pattern"));
                        }

                        if start_pattern.is_none() {
                            return Err(Error::missing_field("start_pattern"));
                        }

                        if mode.is_none() {
                            return Err(Error::missing_field("mode"));
                        }

                        Ok(Config {
                            timeout,
                            parser: Parser::Custom {
                                condition_pattern: condition_pattern.unwrap(),
                                start_pattern: start_pattern.unwrap(),
                                mode: mode.unwrap(),
                            },
                        })
                    }

                    Some(CRI_PARSER) => Ok(Config {
                        timeout,
                        parser: Parser::Cri,
                    }),

                    Some(DOCKER_PARSER) => Ok(Config {
                        timeout,
                        parser: Parser::Docker,
                    }),

                    Some(GO_PARSER) => Ok(Config {
                        timeout,
                        parser: Parser::Go,
                    }),

                    Some(JAVA_PARSER) => Ok(Config {
                        timeout,
                        parser: Parser::Java,
                    }),

                    Some(NO_INDENT) => Ok(Config {
                        timeout,
                        parser: Parser::NoIndent,
                    }),

                    _ => unreachable!(),
                }
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}

impl Serialize for Config {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_struct("Config", 2)?;

        if self.timeout != DEFAULT_TIMEOUT {
            serializer.serialize_field("timeout", &humanize::duration::duration(&self.timeout))?;
        }

        match &self.parser {
            Parser::Cri => {
                serializer.serialize_field("parser", "cri")?;
            }
            Parser::Docker => {
                serializer.serialize_field("parser", "docker")?;
            }
            Parser::Go => {
                serializer.serialize_field("parser", "go")?;
            }
            Parser::Java => {
                serializer.serialize_field("parser", "java")?;
            }
            Parser::NoIndent => {
                serializer.serialize_field("parser", "no_indent")?;
            }
            Parser::Custom {
                condition_pattern,
                start_pattern,
                mode,
            } => {
                serializer.serialize_field("start_pattern", start_pattern.as_str())?;
                serializer.serialize_field("condition_pattern", condition_pattern.as_str())?;
                serializer.serialize_field("mode", mode.as_str())?;
            }
        }

        serializer.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_deserialization() {
        let tests = [
            // simple word
            (
                "cri",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Cri,
                }),
            ),
            (
                "docker",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Docker,
                }),
            ),
            (
                "go",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Go,
                }),
            ),
            (
                "java",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Java,
                }),
            ),
            (
                "no_indent",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::NoIndent,
                }),
            ),
            // parser with timeout
            (
                "parser: cri\ntimeout: 6s",
                Ok(Config {
                    timeout: Duration::from_secs(6),
                    parser: Parser::Cri,
                }),
            ),
            // parser without timeout
            (
                "parser: cri",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Cri,
                }),
            ),
            // custom parser without timeout
            (
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\nmode: continue_through",
                Ok(Config {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Custom {
                        condition_pattern: Regex::new(".*").unwrap(),
                        start_pattern: Regex::new(".*").unwrap(),
                        mode: Mode::ContinueThrough,
                    },
                }),
            ),
            // custom parser with timeout
            (
                r#"
                parser: custom
                start_pattern: .*
                condition_pattern: .*
                mode: continue_through
                timeout: 6s"#,
                Ok(Config {
                    timeout: Duration::from_secs(6),
                    parser: Parser::Custom {
                        condition_pattern: Regex::new(".*").unwrap(),
                        start_pattern: Regex::new(".*").unwrap(),
                        mode: Mode::ContinueThrough,
                    },
                }),
            ),
            // Errors
            //
            // Note: the serde_yaml cannot indicate error position which is horrible,
            //   see: https://github.com/dtolnay/serde-yaml/issues/153

            // unknown parser
            (
                "parser: abc",
                Err(
                    "unknown variant `parser`, expected one of `cri`, `docker`, `go`, `java`, `no_indent`, `custom`",
                ),
            ),
            // unknown mode
            (
                r#"
                parser: custom
                start_pattern: .*
                condition_pattern: .*
                mode: foo
                timeout: 6s"#,
                Err(
                    "unknown variant `mode`, expected one of `continue_through`, `continue_past`, `halt_before`, `halt_with` at line 2 column 17",
                ),
            ),
            // invalid timeout
            (
                r#"
                parser: custom
                start_pattern: .*
                condition_pattern: .*
                mode: continue_through
                timeout: 100"#,
                Err(
                    "invalid value: string \"100\", expected something like 5s, 10s at line 2 column 17",
                ),
            ),
            // missing start_pattern
            (
                r#"
                parser: custom
                condition_pattern: .*
                mode: continue_through
                timeout: 6s"#,
                Err("missing field `start_pattern` at line 2 column 17"),
            ),
            // missing condition_pattern
            (
                r#"
                parser: custom
                start_pattern: .*
                mode: continue_through
                timeout: 6s"#,
                Err("missing field `condition_pattern` at line 2 column 17"),
            ),
            // missing mode
            (
                r#"
                parser: custom
                start_pattern: .*
                condition_pattern: .*
                timeout: 6s"#,
                Err("missing field `mode` at line 2 column 17"),
            ),
        ];

        for (input, want) in tests {
            let want = want.map_err(|err| err.to_string());
            let deserialized: Result<Config, _> =
                serde_yaml::from_str(input).map_err(|err| err.to_string());

            assert_eq!(deserialized, want, "input: {}", input)
        }
    }

    #[test]
    fn json_deserialization() {
        let tests = [(
            r#" "cri" "#,
            Ok(Config {
                timeout: DEFAULT_TIMEOUT,
                parser: Parser::Cri,
            }),
        )];

        for (input, want) in tests {
            let deserialized: Result<Config, _> =
                serde_json::from_str(input).map_err(|err| err.to_string());

            assert_eq!(deserialized, want, "input: {}", input)
        }
    }
}
