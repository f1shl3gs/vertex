mod get;
mod get_mut;
mod insert;
mod remove;

use std::borrow::Borrow;
use std::collections::BTreeMap;

pub use get::get;
pub use get_mut::get_mut;
pub use insert::insert;
pub use remove::remove;

use crate::Value;

pub trait ValueCollection {
    type Key: Borrow<Self::BorrowedKey>;
    type BorrowedKey: ?Sized;

    fn get_value(&self, key: &Self::BorrowedKey) -> Option<&Value>;
    fn get_mut_value(&mut self, key: &Self::BorrowedKey) -> Option<&mut Value>;
    fn insert_value(&mut self, key: Self::Key, value: Value) -> Option<Value>;
    fn remove_value(&mut self, key: &Self::BorrowedKey) -> Option<Value>;
    fn is_empty_collection(&self) -> bool;
}

impl ValueCollection for Value {
    type Key = ();
    type BorrowedKey = ();

    fn get_value(&self, _key: &Self::BorrowedKey) -> Option<&Value> {
        Some(self)
    }

    fn get_mut_value(&mut self, _key: &Self::BorrowedKey) -> Option<&mut Value> {
        Some(self)
    }

    fn insert_value(&mut self, _key: Self::Key, value: Value) -> Option<Value> {
        Some(std::mem::replace(self, value))
    }

    fn remove_value(&mut self, _key: &Self::BorrowedKey) -> Option<Value> {
        match self {
            Self::Object(map) => return Some(Self::Object(std::mem::take(map))),
            Self::Array(array) => return Some(Self::Array(std::mem::take(array))),
            _ => {}
        }

        // removing non-collection types replaces it with null
        Some(std::mem::replace(self, Self::Null))
    }

    fn is_empty_collection(&self) -> bool {
        false
    }
}

impl ValueCollection for BTreeMap<String, Value> {
    type Key = String;
    type BorrowedKey = str;

    fn get_value(&self, key: &Self::BorrowedKey) -> Option<&Value> {
        self.get(key)
    }

    fn get_mut_value(&mut self, key: &Self::BorrowedKey) -> Option<&mut Value> {
        self.get_mut(key)
    }

    fn insert_value(&mut self, key: Self::Key, value: Value) -> Option<Value> {
        self.insert(key, value)
    }

    fn remove_value(&mut self, key: &Self::BorrowedKey) -> Option<Value> {
        self.remove(key)
    }

    fn is_empty_collection(&self) -> bool {
        self.is_empty()
    }
}

fn array_index(array: &[Value], index: isize) -> Option<usize> {
    if index >= 0 {
        Some(index as usize)
    } else {
        let index = array.len() as isize + index;
        if index >= 0 {
            Some(index as usize)
        } else {
            None
        }
    }
}

impl ValueCollection for Vec<Value> {
    type Key = isize;
    type BorrowedKey = isize;

    fn get_value(&self, key: &Self::BorrowedKey) -> Option<&Value> {
        array_index(self, *key).and_then(|index| self.get(index))
    }

    fn get_mut_value(&mut self, key: &Self::BorrowedKey) -> Option<&mut Value> {
        array_index(self, *key).and_then(|index| self.get_mut(index))
    }

    fn insert_value(&mut self, key: Self::Key, value: Value) -> Option<Value> {
        if key >= 0 {
            if self.len() <= (key as usize) {
                while self.len() <= (key as usize) {
                    self.push(Value::Null)
                }

                self[key as usize] = value;
                None
            } else {
                Some(std::mem::replace(&mut self[key as usize], value))
            }
        } else {
            let len_required = -key as usize;
            if self.len() < len_required {
                while self.len() < (len_required - 1) {
                    self.insert(0, Value::Null);
                }

                self.insert(0, value);
                None
            } else {
                let index = (self.len() as isize + key) as usize;
                Some(std::mem::replace(&mut self[index], value))
            }
        }
    }

    fn remove_value(&mut self, key: &Self::BorrowedKey) -> Option<Value> {
        if let Some(index) = array_index(self, *key)
            && index < self.len()
        {
            return Some(self.remove(index));
        }

        None
    }

    fn is_empty_collection(&self) -> bool {
        self.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_field() {
        let mut value = Value::from(BTreeMap::default());
        let key = "root";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(value.as_object().unwrap()[key], marker);
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn nested_field() {
        let mut value = Value::from(BTreeMap::default());
        let key = "root.doot";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_object().unwrap()["root"].as_object().unwrap()["doot"],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn double_nested_field() {
        let mut value = Value::from(BTreeMap::default());
        let key = "root.doot.toot";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_object().unwrap()["root"].as_object().unwrap()["doot"]
                .as_object()
                .unwrap()["toot"],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn single_index() {
        let mut value = Value::from(Vec::<Value>::default());
        let key = "[0]";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(value.as_array_unwrap()[0], marker);
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn nested_index() {
        let mut value = Value::from(Vec::<Value>::default());
        let key = "[0][0]";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(value.as_array_unwrap()[0].as_array_unwrap()[0], marker);
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn field_index() {
        let mut value = Value::from(BTreeMap::default());
        let key = "root[0]";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_object().unwrap()["root"].as_array_unwrap()[0],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn index_field() {
        let mut value = Value::from(Vec::<Value>::default());
        let key = "[0].boot";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_array_unwrap()[0].as_object().unwrap()["boot"],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn nested_index_field() {
        let mut value = Value::from(Vec::<Value>::default());
        let key = "[0][0].boot";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_array_unwrap()[0].as_array_unwrap()[0]
                .as_object()
                .unwrap()["boot"],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn field_with_nested_index_field() {
        let mut value = Value::from(BTreeMap::default());
        let key = "root[0][0].boot";
        let mut marker = Value::from(true);
        assert_eq!(value.insert(key, marker.clone()), None);
        assert_eq!(
            value.as_object().unwrap()["root"].as_array_unwrap()[0].as_array_unwrap()[0]
                .as_object()
                .unwrap()["boot"],
            marker
        );
        assert_eq!(value.get(key), Some(&marker));
        assert_eq!(value.get_mut(key), Some(&mut marker));
        assert_eq!(value.remove(key, false), Some(marker));
    }

    #[test]
    fn populated_field() {
        let mut value = Value::from(BTreeMap::default());
        let marker = Value::from(true);
        assert_eq!(value.insert("a[2]", marker.clone()), None);

        let key = "a[0]";
        assert_eq!(value.insert(key, marker.clone()), Some(Value::Null));

        assert_eq!(value.as_object().unwrap()["a"].as_array_unwrap().len(), 3);
        assert_eq!(value.as_object().unwrap()["a"].as_array_unwrap()[0], marker);
        assert_eq!(
            value.as_object().unwrap()["a"].as_array_unwrap()[1],
            Value::Null
        );
        assert_eq!(value.as_object().unwrap()["a"].as_array_unwrap()[2], marker);

        // Replace the value at 0.
        let marker = Value::from(false);
        assert_eq!(value.insert(key, marker.clone()), Some(Value::from(true)));
        assert_eq!(value.as_object().unwrap()["a"].as_array_unwrap()[0], marker);
    }
}
