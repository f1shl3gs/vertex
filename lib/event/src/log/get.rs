use std::collections::BTreeMap;

use super::path_iter::{PathComponent, PathIter};
use super::Value;

/// Returns a reference to a field value specified by the given path.
pub fn get<'a>(fields: &'a BTreeMap<String, Value>, path: &str) -> Option<&'a Value> {
    let mut path_iter = PathIter::new(path);

    match path_iter.next() {
        Some(PathComponent::Key(key)) => match fields.get(key.as_ref()) {
            None => None,
            Some(value) => get_value(value, path_iter),
        },
        _ => None,
    }
}

/// Returns a reference to a field value specified by a path iter.
fn get_value<'a, I>(mut value: &Value, mut path_iter: I) -> Option<&Value>
where
    I: Iterator<Item = PathComponent<'a>>,
{
    loop {
        match (path_iter.next(), value) {
            (None, _) => return Some(value),
            (Some(PathComponent::Key(key)), Value::Object(map)) => match map.get(key.as_ref()) {
                None => return None,
                Some(nested) => value = nested,
            },
            (Some(PathComponent::Index(index)), Value::Array(array)) => match array.get(index) {
                None => return None,
                Some(nested) => value = nested,
            },
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::fields_from_json;
    use serde_json::json;

    #[test]
    fn simple() {
        let fields = fields_from_json(json!({
            "field": 123,
        }));
        assert_eq!(get(&fields, "field"), Some(&Value::Int64(123)))
    }

    #[test]
    fn nested() {
        let fields = fields_from_json(json!({
            "a": {
                "b": {
                    "c": 5
                },
                "d": 4,
                "array": [
                    null,
                    3,
                    {
                        "x": 1
                    },
                    [2]
                ]
            }
        }));

        let tests = vec![
            ("a.b.c", Some(Value::Int64(5))),
            ("a.d", Some(Value::Int64(4))),
            ("a.array[1]", Some(Value::Int64(3))),
            ("a.array[2].x", Some(Value::Int64(1))),
            ("a.array[3][0]", Some(Value::Int64(2))),
            ("a.array[3][1]", None),
            ("a.x", None),
            ("z", None),
            (".123", None),
            ("", None),
        ];

        for (path, want) in tests {
            assert_eq!(get(&fields, path), want.as_ref(), "{}", path)
        }
    }
}
