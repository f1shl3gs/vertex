use serde::Deserialize;
use value::{OwnedSegment, OwnedValuePath, Value};

/// Endpoint is a service that can be contacted remotely
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Endpoint {
    /// ID uniquely identifies this endpoint
    pub id: String,
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
            "target" => {
                if segments.is_empty() {
                    return Some(Value::Bytes(self.target.clone().into()));
                }
            }
            "env" => {}
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

        let path = parse_value_path("env.foo").unwrap();
        assert_eq!(endpoint.get(&path), Some(Value::Bytes("bar".into())));
    }
}
