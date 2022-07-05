use std::collections::BTreeMap;

use super::Value;
use crate::log::path_iter::{PathComponent, PathIter};

/// Checks whether a field specified by a given path is present
pub fn contains(fields: &BTreeMap<String, Value>, path: &str) -> bool {
    let mut path_iter = PathIter::new(path);

    match path_iter.next() {
        Some(PathComponent::Key(key)) => match fields.get(key.as_ref()) {
            None => false,
            Some(value) => value_contains(value, path_iter),
        },
        _ => false,
    }
}

fn value_contains<'a, I>(mut value: &Value, mut path_iter: I) -> bool
where
    I: Iterator<Item = PathComponent<'a>>,
{
    loop {
        value = match (path_iter.next(), value) {
            (None, _) => return true,
            (Some(PathComponent::Key(key)), Value::Object(map)) => match map.get(key.as_ref()) {
                None => return false,
                Some(nested) => nested,
            },
            (Some(PathComponent::Index(index)), Value::Array(array)) => match array.get(index) {
                None => return false,
                Some(nested) => nested,
            },
            _ => return false,
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
            "field": 123
        }));

        assert!(contains(&fields, "field"))
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
                        "x": 5
                    },
                    [5]
                ]
            }
        }));

        let tests = vec![
            ("a.b.c", true),
            ("a.d", true),
            ("a.array[1]", true),
            ("a.array[2].x", true),
            ("a.array[3][0]", true),
            ("a.array[3][1]", false),
            ("a.x", false),
            ("z", false),
            (".123", false),
            ("", false),
        ];

        for (path, want) in tests {
            assert_eq!(contains(&fields, path), want);
        }
    }
}
