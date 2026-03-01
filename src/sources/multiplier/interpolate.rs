use std::collections::BTreeMap;
use std::sync::LazyLock;

use framework::observe::Endpoint;
use regex::Regex;
use serde_json::{Number, Value as JsonValue};
use value::{Value, parse_value_path};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Utf8(std::str::Utf8Error),
    #[error("invalid path \"{0}\"")]
    InvalidPath(String),
    #[error("value not found {0}")]
    ValueNotFound(String),
    #[error("deserialize config failed, err: {0}")]
    Deserialize(serde_json::error::Error),
}

/// build source interpolate the template value, and build the source
/// ```yaml
/// id: {{ $id }}
/// target: {{ $target }}
/// env:
///   {{ $env.foo }}: {{ $env.bar }}
/// ```
pub fn interpolate(input: &Value, endpoint: &Endpoint) -> Result<JsonValue, Error> {
    match input {
        Value::Object(map) => interpolate_map(map, endpoint),
        Value::Array(array) => array
            .iter()
            .map(|input| interpolate(input, endpoint))
            .collect::<Result<Vec<_>, _>>()
            .map(JsonValue::Array),
        Value::Bytes(data) => {
            let input = std::str::from_utf8(data).map_err(Error::Utf8)?;
            interpolate_value(input, endpoint)
        }
        Value::Integer(i) => Ok(JsonValue::Number(Number::from(*i))),
        Value::Float(f) => Ok(JsonValue::Number(Number::from_f64(*f).unwrap())),
        Value::Boolean(b) => Ok(JsonValue::Bool(*b)),
        Value::Timestamp(ts) => Ok(JsonValue::String(ts.to_string())),
        Value::Null => Ok(JsonValue::Null),
    }
}

fn interpolate_map(
    input: &BTreeMap<String, Value>,
    endpoint: &Endpoint,
) -> Result<JsonValue, Error> {
    let mut output = serde_json::map::Map::with_capacity(input.len());

    for (key, value) in input {
        let key = interpolate_string(key, endpoint)?;
        let value = interpolate(value, endpoint)?;
        output.insert(key, value);
    }

    Ok(JsonValue::Object(output))
}

static VARIABLE_INTERPOLATION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"\$\{\{\s*(\S+)\s*}}"#).unwrap());

fn interpolate_string(input: &str, endpoint: &Endpoint) -> Result<String, Error> {
    let mut replaced = String::with_capacity(input.len());
    let mut last_match = 0;
    for caps in VARIABLE_INTERPOLATION_REGEX.captures_iter(input) {
        let m = caps.get(0).unwrap();
        replaced.push_str(&input[last_match..m.start()]);

        let path = caps.get(1).unwrap().as_str();
        let value_path =
            parse_value_path(path).map_err(|_err| Error::InvalidPath(path.to_string()))?;

        match endpoint.get(&value_path) {
            Some(value) => {
                replaced.push_str(value.to_string().as_str());
                last_match = m.end();
            }
            None => return Err(Error::ValueNotFound(path.to_string())),
        }
    }

    replaced.push_str(&input[last_match..]);

    Ok(replaced)
}

fn interpolate_value(input: &str, endpoint: &Endpoint) -> Result<JsonValue, Error> {
    let matches = VARIABLE_INTERPOLATION_REGEX.captures_iter(input);
    // no match
    if matches.count() == 0 {
        return Ok(JsonValue::String(input.to_string()));
    }

    let mut replaced = String::with_capacity(input.len());
    let mut last_match = 0;

    for caps in VARIABLE_INTERPOLATION_REGEX.captures_iter(input) {
        let m = caps.get(0).unwrap();
        replaced.push_str(&input[last_match..m.start()]);

        let path = caps.get(1).unwrap().as_str();
        let value_path =
            parse_value_path(path).map_err(|_err| Error::InvalidPath(path.to_string()))?;

        match endpoint.get(&value_path) {
            Some(value) => {
                if m.as_str().len() == input.len() {
                    return interpolate(&value, endpoint);
                }

                replaced.push_str(value.to_string_lossy().as_ref());
                last_match = m.end();
            }
            None => return Err(Error::ValueNotFound(path.to_string())),
        }
    }

    replaced.push_str(&input[last_match..]);

    Ok(JsonValue::String(replaced))
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;
    use value::value;

    fn mock_endpoint() -> Endpoint {
        Endpoint {
            id: "1234".to_string(),
            typ: "mock".into(),
            target: "127.0.0.1".to_string(),
            details: value!({
                "foo": "bar",
                "int": 1,
                "f64": 2.2,
                "array": [1, 2, 3],
                "null": null,
                "map": {
                    "key1": {},
                    "key2": 1
                }
            }),
        }
    }

    #[test]
    fn object() {
        let input = value!({
            "id": "${{ id }}",
            "target": "${{ target }}",
            "details": "${{ details }}",
        });
        let output = interpolate(&input, &mock_endpoint()).unwrap();
        assert_eq!(
            output,
            json!({
                "id": "1234",
                "target": "127.0.0.1",
                "details": {
                    "foo": "bar",
                    "int": 1,
                    "f64": 2.2,
                    "array": [1, 2, 3],
                    "null": null,
                    "map": {
                        "key1": {},
                        "key2": 1
                    }
                }
            })
        );
    }

    #[test]
    fn string() {
        let yaml_text =
            "type: http_check\ninterval: 15s\ntargets:\n  - url: http://${{ target }}\n";
        let input = serde_yaml::from_str::<Value>(yaml_text).unwrap();

        let endpoint = Endpoint {
            id: "".to_string(),
            typ: "mock".into(),
            target: "127.0.0.1:33331".to_string(),
            details: Value::Null,
        };

        let n = interpolate(&input, &endpoint).unwrap();

        println!("{n}");
    }
}
