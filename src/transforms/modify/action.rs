use bytes::{Buf, Bytes};
use event::log::{OwnedTargetPath, Value};
use event::LogRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Error {
    NotFound,
    Convert,

    ParseBool(String),
    ParseJson(serde_json::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Set a key/value pair with path and value, if the path already exists,
    /// this field is overwritten.
    Set { path: OwnedTargetPath, value: Value },
    /// Add a key/value pair with path and value, if the path does not exists.
    Add { path: OwnedTargetPath, value: Value },
    /// Remove a key/value pair with path if it exists.
    Remove { path: OwnedTargetPath },
    /// Move a value form `from` to `to` if it exists.
    Move {
        from: OwnedTargetPath,
        to: OwnedTargetPath,
    },

    /// Convert a log field from bool/f64 to i64.
    ToInteger { path: OwnedTargetPath },
    /// Convert a log field from i64/f64/string to bool.
    ToBool { path: OwnedTargetPath },
    /// Convert a log filed to string
    ToString { path: OwnedTargetPath },

    /// Parse a string field, and put it to path or target(when it is not provide).
    ParseJson {
        path: OwnedTargetPath,
        target: Option<OwnedTargetPath>,
    },

    /// Returns a slice of value from start to end
    Substr {
        path: OwnedTargetPath,
        /// Offset from start, count from zero.
        #[serde(default)]
        start: usize,
        /// Length from offset, keeping the first `length` bytes and dropping the
        /// rest. If `length` is greater than the bytes's current length, this has no
        /// effect.
        #[serde(default)]
        length: Option<usize>,
    },
}

impl Action {
    pub fn apply(&self, log: &mut LogRecord) -> Result<(), Error> {
        match self {
            Action::Set { path, value } => {
                log.insert(path, value.clone());
            }
            Action::Add { path, value } => {
                if !log.contains(path) {
                    log.insert(path, value.clone());
                }
            }
            Action::Remove { path } => {
                log.remove_prune(path, true);
            }
            Action::Move { from, to } => {
                if let Some(value) = log.remove_prune(from, true) {
                    log.insert(to, value);
                }
            }

            // Converts
            Action::ToInteger { path } => {
                if let Some(value) = log.get_mut(path) {
                    match value {
                        Value::Integer(_i) => {
                            // it is already an integer so nothing need to do
                        }
                        Value::Boolean(b) => {
                            let new = if *b { 1 } else { 0 };
                            *value = Value::Integer(new);
                        }
                        Value::Float(f) => *value = Value::Integer(*f as i64),
                        Value::Bytes(b) => {
                            let s = String::from_utf8_lossy(b);
                            let new = s.parse::<i64>().map_err(|_err| Error::Convert)?;
                            *value = Value::Integer(new);
                        }
                        _ => return Err(Error::Convert),
                    }
                }
            }
            Action::ToBool { path } => {
                if let Some(value) = log.get_mut(path) {
                    match value {
                        Value::Boolean(_b) => {
                            // it is Value::Boolean already, nothing need to do
                        }
                        Value::Integer(i) => {
                            *value = Value::Boolean(*i != 0);
                        }
                        Value::Float(f) => {
                            *value = Value::Boolean(*f != 0.0);
                        }
                        Value::Bytes(b) => {
                            let s = String::from_utf8_lossy(b).to_lowercase();
                            let new_value = match s.as_str() {
                                "true" | "t" | "yes" | "y" | "on" => true,
                                "false" | "f" | "no" | "n" | "off" => false,
                                _ => {
                                    if let Ok(n) = s.parse::<i64>() {
                                        n != 0
                                    } else {
                                        // Do the case conversion only if simple matches fail,
                                        // since this operation can be expensive.
                                        match s.as_str() {
                                            "true" | "t" | "yes" | "y" | "on" => true,
                                            "false" | "f" | "no" | "n" | "off" => false,
                                            _ => return Err(Error::ParseBool(s)),
                                        }
                                    }
                                }
                            };

                            *value = Value::Boolean(new_value);
                        }
                        _ => return Err(Error::Convert),
                    }
                }
            }
            Action::ToString { path } => match log.get_mut(path) {
                Some(value) => {
                    let b = Bytes::from(value.to_string_lossy().into_owned());
                    *value = Value::Bytes(b);
                }
                None => return Err(Error::NotFound),
            },

            // Parse
            Action::ParseJson { path, target } => match log.remove(path) {
                Some(value) => {
                    let new_value = match value {
                        Value::Bytes(b) => serde_json::from_slice(&b).map_err(Error::ParseJson)?,
                        v => v,
                    };

                    match target {
                        Some(target) => log.insert(target, new_value),
                        None => log.insert(path, new_value),
                    };
                }
                None => return Err(Error::NotFound),
            },

            // Strings
            Action::Substr {
                path,
                start,
                length,
            } => match log.get_mut(path) {
                Some(value) => {
                    if let Value::Bytes(value) = value {
                        if *start != 0 {
                            let offset = value.remaining().min(*start);
                            value.advance(offset);
                        }

                        if let Some(length) = length {
                            value.truncate(*length);
                        }
                    } else {
                        return Err(Error::Convert);
                    }
                }
                None => return Err(Error::NotFound),
            },
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use event::fields;
    use event::log::path::parse_target_path;

    use super::*;

    #[test]
    fn apply() {
        for (name, init, action, want) in [
            (
                "set",
                None,
                Action::Set {
                    path: parse_target_path(".foo").unwrap(),
                    value: "bar".into(),
                },
                fields!(
                    "msg" => "data",
                    "foo" => "bar"
                ),
            ),
            (
                "set and overwrite",
                None,
                Action::Set {
                    path: parse_target_path(".msg").unwrap(),
                    value: "bar".into(),
                },
                fields!(
                    "msg" => "bar"
                ),
            ),
            (
                "add",
                None,
                Action::Add {
                    path: parse_target_path(".foo").unwrap(),
                    value: "bar".into(),
                },
                fields!(
                    "msg" => "data",
                    "foo" => "bar"
                ),
            ),
            (
                "add exists field",
                None,
                Action::Add {
                    path: parse_target_path(".msg").unwrap(),
                    value: "bar".into(),
                },
                fields!(
                    "msg" => "data",
                ),
            ),
            (
                "remove",
                None,
                Action::Remove {
                    path: parse_target_path(".msg").unwrap(),
                },
                fields!(),
            ),
            (
                "move",
                None,
                Action::Move {
                    from: parse_target_path(".msg").unwrap(),
                    to: parse_target_path(".message").unwrap(),
                },
                fields!(
                    "message" => "data"
                ),
            ),
            (
                "to integer",
                Some(fields!("int" => "123")),
                Action::ToInteger {
                    path: parse_target_path(".int").unwrap(),
                },
                fields!(
                    "int" => 123
                ),
            ),
            (
                "1i64 to bool",
                Some(fields!("bool" => 1)),
                Action::ToBool {
                    path: parse_target_path(".bool").unwrap(),
                },
                fields!(
                    "bool" => true
                ),
            ),
            (
                "0i64 to bool",
                Some(fields!("bool" => 0)),
                Action::ToBool {
                    path: parse_target_path(".bool").unwrap(),
                },
                fields!(
                    "bool" => false
                ),
            ),
            (
                "1f64 to bool",
                Some(fields!("bool" => 1.0)),
                Action::ToBool {
                    path: parse_target_path(".bool").unwrap(),
                },
                fields!(
                    "bool" => true
                ),
            ),
            (
                "0f64 to bool",
                Some(fields!("bool" => 0.0)),
                Action::ToBool {
                    path: parse_target_path(".bool").unwrap(),
                },
                fields!(
                    "bool" => false
                ),
            ),
            (
                "to string",
                Some(fields!("int" => 1)),
                Action::ToString {
                    path: parse_target_path(".int").unwrap(),
                },
                fields!(
                    "int" => "1"
                )
            ),
            (
                "parse json",
                Some(fields!("raw" => r#"{"foo": "bar"}"#)),
                Action::ParseJson {
                    path: parse_target_path(".raw").unwrap(),
                    target: None,
                },
                fields!(
                    "raw" => fields!(
                        "foo" => "bar"
                    )
                ),
            ),
            (
                "parse json with target",
                Some(fields!("raw" => r#"{"foo": "bar"}"#)),
                Action::ParseJson {
                    path: parse_target_path(".raw").unwrap(),
                    target: Some(parse_target_path(".").unwrap()),
                },
                fields!("foo" => "bar"),
            ),
        ] {
            let fields = init.unwrap_or(fields!("msg" => "data"));
            let mut log = LogRecord::from(fields);
            action.apply(&mut log).unwrap();
            assert_eq!(log.value(), &Value::from(want), "\nTest \"{name}\" failed");
        }
    }
}
