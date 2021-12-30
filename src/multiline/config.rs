use std::fmt::Formatter;
use std::time::Duration;

use regex::bytes::Regex;
use serde::de::{Error, MapAccess};
use serde::{de, Deserializer, Serializer};

use super::aggregate::Mode;

const CRI_PARSER: &str = "cri";
const DOCKER_PARSER: &str = "docker";
const GO_PARSER: &str = "go";
const JAVA_PARSER: &str = "java";
const NOINDENT: &str = "noindent";
const CUSTOM_PARSER: &str = "custom";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub enum Parser {
    Cri,
    Docker,
    Go,
    Java,
    NoIndent,

    Custom {
        condition_pattern: Regex,
        start_pattern: Regex,
        mode: Mode,
    },
}

impl PartialEq for Parser {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Cri => match other {
                Parser::Cri => true,
                _ => false,
            },
            Self::Docker => match other {
                Parser::Docker => true,
                _ => false,
            },
            Self::Go => match other {
                Parser::Go => true,
                _ => false,
            },
            Self::Java => match other {
                Parser::Java => true,
                _ => false,
            },
            Self::NoIndent => match other {
                Parser::NoIndent => true,
                _ => false,
            },
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
///     timeout: 10s
/// ```
///
/// Custom type, except `timeout` all field is required
/// ```yaml
/// multiline:
///     parser: custom
///     timeout: 5s
///     start_pattern:  ^[^\s]
///     condition_pattern: ^[\s]+
///     mode: continue_through
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct MultilineConfig {
    pub timeout: Duration,
    pub parser: Parser,
}

/*
impl MultilineConfig {
    pub fn parser<T: Rule>(&self) -> T {
        match self.parser {
            Parser::Cri => super::preset::Cri {},
            Parser::NoIndent => super::preset::NoIndent {},
            _ => unreachable!(),
        }
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}
*/

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
                    "cri" => Ok(MultilineConfig {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Cri,
                    }),
                    "docker" => Ok(MultilineConfig {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Docker,
                    }),
                    "go" => Ok(MultilineConfig {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Go,
                    }),
                    "java" => Ok(MultilineConfig {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::Java,
                    }),
                    "noindent" => Ok(MultilineConfig {
                        timeout: DEFAULT_TIMEOUT,
                        parser: Parser::NoIndent,
                    }),
                    _ => Err(de::Error::invalid_value(
                        de::Unexpected::Str(v),
                        &r#"cri, docker, go or java"#,
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
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    match key.as_str() {
                        "timeout" => {
                            if timeout.is_some() {
                                return Err(de::Error::duplicate_field("timeout"));
                            }

                            let v =
                                humanize::duration_std::parse_duration(&value).map_err(|_err| {
                                    de::Error::invalid_value(
                                        de::Unexpected::Str(&value),
                                        &r#"something like 5s, 10s"#,
                                    )
                                })?;

                            timeout = Some(v);
                        }
                        "start_pattern" => {
                            if start_pattern.is_some() {
                                return Err(de::Error::duplicate_field("start_pattern"));
                            }

                            let re = Regex::new(&value).map_err(|_err| {
                                de::Error::invalid_value(
                                    de::Unexpected::Str(&value),
                                    &r#"regex is expected"#,
                                )
                            })?;

                            start_pattern = Some(re);
                        }

                        "condition_pattern" => {
                            if condition_pattern.is_some() {
                                return Err(de::Error::duplicate_field("condition_pattern"));
                            }

                            let re = Regex::new(&value).map_err(|_err| {
                                de::Error::invalid_value(
                                    de::Unexpected::Str(&value),
                                    &r#"regex is ecpected"#,
                                )
                            })?;

                            condition_pattern = Some(re);
                        }

                        "parser" => {
                            if parser.is_some() {
                                return Err(de::Error::duplicate_field("parser"));
                            }

                            parser = match value.as_str() {
                                CRI_PARSER => Some(CRI_PARSER),
                                DOCKER_PARSER => Some(DOCKER_PARSER),
                                GO_PARSER => Some(GO_PARSER),
                                JAVA_PARSER => Some(JAVA_PARSER),
                                NOINDENT => Some(NOINDENT),
                                CUSTOM_PARSER => Some(CUSTOM_PARSER),
                                _ => {
                                    return Err(de::Error::unknown_variant(
                                        "parser",
                                        &["cri", "docker", "go", "java", "custom"],
                                    ));
                                }
                            }
                        }

                        "mode" => {
                            if mode.is_some() {
                                return Err(de::Error::duplicate_field("mode"));
                            }

                            mode = Some(match value.as_str() {
                                "continue_through" => Mode::ContinueThrough,
                                "continue_past" => Mode::ContinuePast,
                                "halt_before" => Mode::HaltBefore,
                                "halt_with" => Mode::HaltWith,
                                _ => {
                                    return Err(de::Error::unknown_variant(
                                        "mode",
                                        &[
                                            "continue_through",
                                            "continue_past",
                                            "halt_before",
                                            "halt_with",
                                        ],
                                    ))
                                }
                            });
                        }

                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
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
                    None => return Err(de::Error::missing_field("parser")),

                    Some(CUSTOM_PARSER) => {
                        if condition_pattern.is_none() {
                            return Err(de::Error::missing_field("condition_pattern"));
                        }

                        if start_pattern.is_none() {
                            return Err(de::Error::missing_field("start_pattern"));
                        }

                        if mode.is_none() {
                            return Err(de::Error::missing_field("mode"));
                        }

                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::Custom {
                                condition_pattern: condition_pattern.unwrap(),
                                start_pattern: start_pattern.unwrap(),
                                mode: mode.unwrap(),
                            },
                        });
                    }

                    Some(CRI_PARSER) => {
                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::Cri,
                        });
                    }

                    Some(DOCKER_PARSER) => {
                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::Docker,
                        });
                    }

                    Some(GO_PARSER) => {
                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::Go,
                        });
                    }

                    Some(JAVA_PARSER) => {
                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::Java,
                        });
                    }

                    Some(NOINDENT) => {
                        return Ok(MultilineConfig {
                            timeout,
                            parser: Parser::NoIndent,
                        })
                    }

                    _ => unreachable!(),
                };
            }
        }

        deserializer.deserialize_any(StringOrMap)
    }
}

impl serde::ser::Serialize for MultilineConfig {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unreachable!()
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
                r#"cri"#,
                Ok(MultilineConfig {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Cri,
                }),
            ),
            (
                r#"docker"#,
                Ok(MultilineConfig {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Docker,
                }),
            ),
            (
                r#"go"#,
                Ok(MultilineConfig {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Go,
                }),
            ),
            (
                r#"java"#,
                Ok(MultilineConfig {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Java,
                }),
            ),
            // parser with timeout
            (
                "parser: cri\ntimeout: 6s",
                Ok(MultilineConfig {
                    timeout: Duration::from_secs(6),
                    parser: Parser::Cri,
                }),
            ),
            // parser without timeout
            (
                r#"parser: cri"#,
                Ok(MultilineConfig {
                    timeout: DEFAULT_TIMEOUT,
                    parser: Parser::Cri,
                }),
            ),
            // custom parser without timeout
            (
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\nmode: continue_through",
                Ok(MultilineConfig {
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
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\nmode: continue_through\ntimeout: 6s",
                Ok(MultilineConfig {
                    timeout: Duration::from_secs(6),
                    parser: Parser::Custom {
                        condition_pattern: Regex::new(".*").unwrap(),
                        start_pattern: Regex::new(".*").unwrap(),
                        mode: Mode::ContinueThrough
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
                Err("unknown variant `parser`, expected one of `cri`, `docker`, `go`, `java`, `custom` at line 1 column 7")
            ),
            // unknown mode
            (
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\nmode: foo\ntimeout: 6s",
                Err("unknown variant `mode`, expected one of `continue_through`, `continue_past`, `halt_before`, `halt_with` at line 1 column 7")
            ),
            // invalid timeout
            (
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\nmode: continue_through\ntimeout: 100",
                Err("invalid value: string \"100\", expected something like 5s, 10s at line 1 column 7")
            ),
            // missing start_pattern
            (
                "parser: custom\ncondition_pattern: .*\nmode: continue_through\ntimeout: 6s",
                Err("missing field `start_pattern` at line 1 column 7")
            ),
            // missing condition_pattern
            (
                "parser: custom\nstart_pattern: .*\nmode: continue_through\ntimeout: 6s",
                Err("missing field `condition_pattern` at line 1 column 7"),
            ),
            // missing mode
            (
                "parser: custom\nstart_pattern: .*\ncondition_pattern: .*\ntimeout: 6s",
                Err("missing field `mode` at line 1 column 7")
            )
        ];

        for (input, want) in tests {
            let want = want.map_err(|err| err.to_string());
            let deserialized: Result<MultilineConfig, _> =
                serde_yaml::from_str(input).map_err(|err| err.to_string());

            assert_eq!(deserialized, want, "input: {}", input)
        }
    }

    #[test]
    fn json_deserialization() {
        let tests = [(
            r#" "cri" "#,
            Ok(MultilineConfig {
                timeout: DEFAULT_TIMEOUT,
                parser: Parser::Cri,
            }),
        )];

        for (input, want) in tests {
            let deserialized: Result<MultilineConfig, _> =
                serde_json::from_str(input).map_err(|err| err.to_string());

            assert_eq!(deserialized, want, "input: {}", input)
        }
    }
}
