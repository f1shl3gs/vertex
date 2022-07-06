use std::collections::BTreeMap;
use std::iter::Peekable;

use super::path_iter::{PathComponent, PathIter};
use super::Value;

/// Inserts fields value using a path specified using `a.b[1].c` notation.
pub fn insert(fields: &mut BTreeMap<String, Value>, path: &str, value: Value) -> Option<Value> {
    map_insert(fields, PathIter::new(path).peekable(), value)
}

fn map_insert<'a, I>(
    fields: &mut BTreeMap<String, Value>,
    mut path_iter: Peekable<I>,
    value: Value,
) -> Option<Value>
where
    I: Iterator<Item = PathComponent<'a>>,
{
    match (path_iter.next(), path_iter.peek()) {
        (Some(PathComponent::Key(current)), None) => fields.insert(current.into_owned(), value),
        (Some(PathComponent::Key(current)), Some(PathComponent::Key(_))) => {
            if let Some(Value::Object(map)) = fields.get_mut(current.as_ref()) {
                map_insert(map, path_iter, value)
            } else {
                let mut map = BTreeMap::new();
                map_insert(&mut map, path_iter, value);
                fields.insert(current.into_owned(), Value::Object(map))
            }
        }
        (Some(PathComponent::Key(current)), Some(&PathComponent::Index(next))) => {
            if let Some(Value::Array(array)) = fields.get_mut(current.as_ref()) {
                array_insert(array, path_iter, value)
            } else {
                let mut array = Vec::with_capacity(next + 1);
                array_insert(&mut array, path_iter, value);
                fields.insert(current.into_owned(), Value::Array(array))
            }
        }
        _ => None,
    }
}

fn array_insert<'a, I>(
    array: &mut Vec<Value>,
    mut path_iter: Peekable<I>,
    value: Value,
) -> Option<Value>
where
    I: Iterator<Item = PathComponent<'a>>,
{
    match (path_iter.next(), path_iter.peek()) {
        (Some(PathComponent::Index(current)), None) => {
            while array.len() <= current {
                array.push(Value::Null);
            }

            Some(std::mem::replace(&mut array[current], value))
        }

        (Some(PathComponent::Index(current)), Some(PathComponent::Key(_))) => {
            if let Some(Value::Object(map)) = array.get_mut(current) {
                map_insert(map, path_iter, value)
            } else {
                let mut map = BTreeMap::new();
                map_insert(&mut map, path_iter, value);
                while array.len() <= current {
                    array.push(Value::Null)
                }
                Some(std::mem::replace(&mut array[current], Value::Object(map)))
            }
        }

        (Some(PathComponent::Index(current)), Some(PathComponent::Index(next))) => {
            if let Some(Value::Array(temp)) = array.get_mut(current) {
                array_insert(temp, path_iter, value)
            } else {
                let mut temp_array = Vec::with_capacity(next + 1);
                array_insert(&mut temp_array, path_iter, value);
                while array.len() <= current {
                    array.push(Value::Null)
                }
                Some(std::mem::replace(
                    &mut array[current],
                    Value::Array(temp_array),
                ))
            }
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::fields_from_json;
    use serde_json::json;

    #[test]
    fn test_insert() {
        let tests = vec![
            (
                "a.b.c",
                Value::Int64(3),
                json!({
                    "a": {
                        "b": {
                            "c": 3
                        }
                    }
                }),
            ),
            (
                "a.b[0].c[2]",
                Value::Int64(10),
                json!({
                    "a": {
                        "b": [
                            {
                                "c": [
                                    null,
                                    null,
                                    10
                                ]
                            }
                        ]
                    }
                }),
            ),
        ];

        for (path, value, want) in tests {
            let mut fields = BTreeMap::new();
            insert(&mut fields, path, value);
            assert_eq!(fields, fields_from_json(want))
        }
    }

    #[test]
    fn test_inserts() {
        let mut fields = BTreeMap::new();
        insert(&mut fields, "a.b[0].c[2]", Value::Int64(10));
        insert(&mut fields, "a.b[0].c[0]", Value::Int64(5));

        let want = fields_from_json(json!({
            "a": {
                "b": [
                    {
                        "c": [
                            5,
                            null,
                            10
                        ]
                    }
                ]
            }
        }));

        assert_eq!(fields, want);
    }
}
