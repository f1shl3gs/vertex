use std::borrow::Cow;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use value::{OwnedSegment, OwnedValuePath, Value};

/// Endpoint is a service that can be contacted remotely
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Endpoint {
    /// ID uniquely identifies this endpoint
    pub id: String,
    /// Type of the Endpoint, e.g. service, pod, container, etc
    #[serde(rename = "type")]
    pub typ: Cow<'static, str>,
    /// Target is an IP address or hostname of the endpoint.
    /// It can also be a hostname/ip:port pair.
    pub target: String,
    /// Details contains additional context about the endpoint such as a Pod or Port
    pub details: Value,
}

impl Endpoint {
    pub fn get(&self, path: &OwnedValuePath) -> Option<Value> {
        let (first, segments) = path.segments.split_first()?;
        let first = match first {
            OwnedSegment::Field(f) => f,
            _ => return None,
        };

        match first.as_str() {
            "id" => {
                if segments.is_empty() {
                    return Some(Value::Bytes(self.id.clone().into()));
                }
            }
            "type" => {
                if segments.is_empty() {
                    let value = match &self.typ {
                        Cow::Borrowed(b) => Bytes::from_static(b.as_bytes()),
                        Cow::Owned(o) => Bytes::from(o.to_string()),
                    };

                    return Some(Value::Bytes(value));
                }
            }
            "target" => {
                if segments.is_empty() {
                    return Some(Value::Bytes(self.target.clone().into()));
                }
            }
            "details" => {}
            _ => return None,
        }

        self.details.get(segments).cloned()
    }
}

#[cfg(test)]
mod tests {
    use value::{parse_value_path, value};

    use super::*;

    #[test]
    fn get() {
        let endpoint = Endpoint {
            id: "1234".to_string(),
            typ: "test".into(),
            target: "127.0.0.1".to_string(),
            details: value!({
                "foo": "bar",
                "arr": [1, 2, 3],
            }),
        };

        let path = parse_value_path("id").unwrap();
        assert_eq!(endpoint.get(&path), Some(Value::Bytes("1234".into())));

        let path = parse_value_path("target").unwrap();
        assert_eq!(endpoint.get(&path), Some(Value::Bytes("127.0.0.1".into())));

        let path = parse_value_path("details.foo").unwrap();
        assert_eq!(endpoint.get(&path), Some(Value::Bytes("bar".into())));
    }
}
