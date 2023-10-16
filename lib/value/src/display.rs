use std::fmt;
use std::fmt::Formatter;

use chrono::SecondsFormat;

use crate::Value;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bytes(b) => write!(
                f,
                "\"{}\"",
                String::from_utf8_lossy(b)
                    .replace('\\', r"\\")
                    .replace('"', r#"\""#)
                    .replace('\n', r"\n")
            ),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Object(map) => {
                let joined = map
                    .iter()
                    .map(|(key, value)| format!(r#""{key}": {value}"#))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{{ {joined} }}")
            }
            Self::Array(array) => {
                let joined = array
                    .iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "[{joined}]")
            }
            Self::Timestamp(ts) => {
                write!(f, "t'{}'", ts.to_rfc3339_opts(SecondsFormat::AutoSi, true))
            }
            Self::Null => write!(f, "null"),
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use chrono::DateTime;

    use super::Value;

    #[test]
    fn display_string() {
        assert_eq!(
            Value::Bytes(Bytes::from("Hello, world!")).to_string(),
            r#""Hello, world!""#
        );
    }

    #[test]
    fn display_string_with_backslashes() {
        assert_eq!(
            Value::Bytes(Bytes::from(r"foo \ bar \ baz")).to_string(),
            r#""foo \\ bar \\ baz""#
        );
    }

    #[test]
    fn display_string_with_quotes() {
        assert_eq!(
            Value::Bytes(Bytes::from(r#""Hello, world!""#)).to_string(),
            r#""\"Hello, world!\"""#
        );
    }

    #[test]
    fn display_string_with_newlines() {
        assert_eq!(
            Value::Bytes(Bytes::from("Some\nnew\nlines\n")).to_string(),
            r#""Some\nnew\nlines\n""#
        );
    }

    #[test]
    fn display_integer() {
        assert_eq!(Value::Integer(123).to_string(), "123");
    }

    #[test]
    fn display_float() {
        assert_eq!(Value::Float(123.45).to_string(), "123.45");
    }

    #[test]
    fn display_boolean() {
        assert_eq!(Value::Boolean(true).to_string(), "true");
    }

    #[test]
    fn display_object() {
        assert_eq!(
            Value::Object([("foo".into(), "bar".into())].into()).to_string(),
            r#"{ "foo": "bar" }"#
        );
    }

    #[test]
    fn display_array() {
        assert_eq!(
            Value::Array(
                vec!["foo", "bar"]
                    .into_iter()
                    .map(std::convert::Into::into)
                    .collect()
            )
            .to_string(),
            r#"["foo", "bar"]"#
        );
    }

    #[test]
    fn display_timestamp() {
        assert_eq!(
            Value::Timestamp(
                DateTime::parse_from_rfc3339("2000-10-10T20:55:36Z")
                    .unwrap()
                    .into()
            )
            .to_string(),
            "t'2000-10-10T20:55:36Z'"
        );
    }

    #[test]
    fn display_null() {
        assert_eq!(Value::Null.to_string(), "null");
    }
}
