//! Parse for (Logfmt)[https://brandur.org/logfmt]

use std::collections::BTreeMap;

use bytes::Bytes;
use event::log::Value;
use event::{Events, LogRecord};

use super::{DeserializeError, Deserializer};

// This parser is based on https://github.com/brandur/logfmt
pub(crate) fn parse(line: &str) -> BTreeMap<String, Value> {
    let mut fields = BTreeMap::<String, Value>::new();
    let mut key = None;
    let mut buf = String::new();

    let mut escape = false;
    let mut garbage = false;
    let mut quoted = false;

    for c in line.chars() {
        match (quoted, c) {
            (false, ' ') => {
                if !buf.is_empty() {
                    if !garbage {
                        // the buffer that we just processed is either a value
                        // or a valueless key depending on the current state of
                        // `pair`
                        match key {
                            Some(key) => {
                                fields.insert(key, buf.into());
                            }
                            None => {
                                fields.insert(buf, true.into());
                            }
                        }
                        key = None;
                    }
                    buf = String::new();
                }
                garbage = false;
            }
            (false, '=') => {
                if !buf.is_empty() {
                    key = Some(buf);
                    buf = String::new();
                } else {
                    garbage = true;
                }
            }
            (true, '\\') => {
                escape = true;
            }
            (_, '"') => {
                if escape {
                    buf.push(c);
                    escape = false;
                } else {
                    quoted = !quoted;
                }
            }
            _ => {
                // if the last character we read was an escape, but this
                // character was not a quote, then store the escape back into the
                // buffer
                if escape {
                    buf.push('\\');
                    escape = false;
                }
                buf.push(c);
            }
        }
    }

    // and process one final time at the end of the message to get the last
    // data point
    if !garbage {
        match key {
            Some(key) => {
                let value = if buf.is_empty() {
                    true.into()
                } else {
                    buf.into()
                };

                fields.insert(key, value);
            }
            None => {
                fields.insert(buf, true.into());
            }
        }
    }

    fields
}

/// Deserializer that builds `Event`s from a byte frame containing Logfmt logs
#[derive(Clone, Debug)]
pub struct LogfmtDeserializer;

impl Deserializer for LogfmtDeserializer {
    fn parse(&self, buf: Bytes) -> Result<Events, DeserializeError> {
        let line = std::str::from_utf8(&buf)?;

        let pairs = parse(line);
        let log = LogRecord::from(pairs);

        Ok(log.into())
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;

    #[test]
    fn deserialize() {
        let tests = [
            (
                "a",
                value!({
                    "a": true,
                }),
            ),
            (
                "a=",
                value!({
                    "a": true,
                }),
            ),
            (
                "a= ",
                value!({
                    "a": true,
                }),
            ),
            (
                "a=b",
                value!({
                    "a": "b",
                }),
            ),
            (
                "a=\"b\"",
                value!({
                    "a": "b"
                }),
            ),
            (
                "a=\"f(\\\"b\\\")",
                value!({
                    "a": "f(\"b\")"
                }),
            ),
            (
                "a=\\b",
                value!({
                    "a": "\\b"
                }),
            ),
            (
                "a=1 b=\"bar\" ƒ=2h3s r=\"esc\t\" d x=sf",
                value!({
                    "a": "1",
                    "b": "bar",
                    "ƒ": "2h3s",
                    "r": "esc\t",
                    "d": true,
                    "x": "sf"
                }),
            ),
            (
                r#"foo=bar a=14 baz="hello kitty" cool%story=bro f %^asdf"#,
                value!({
                    "foo": "bar",
                    "a": "14",
                    "baz": "hello kitty",
                    "cool%story": "bro",
                    "f": true,
                    "%^asdf": true,
                }),
            ),
        ];

        let deserializer = LogfmtDeserializer;
        for (input, want) in tests {
            let mut logs = deserializer
                .parse(input.into())
                .unwrap()
                .into_logs()
                .unwrap();
            assert_eq!(logs.len(), 1);

            let log = logs.remove(0);
            let got = log.value();
            assert_eq!(got, &want, "input: {input}\ngot:  {got:?}\nwant: {want:?}");
        }
    }
}
