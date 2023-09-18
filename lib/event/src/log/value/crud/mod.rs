#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

mod get;
mod get_mut;
mod insert;
mod remove;

use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;

use lookup::BorrowedSegment;

pub use get::get;
pub use get_mut::get_mut;
pub use insert::insert;
pub use remove::remove;

use super::Value;

pub trait ValueCollection {
    type BorrowedKey: ?Sized;
    type Key: Borrow<Self::BorrowedKey>;

    fn get_value(&self, key: &Self::BorrowedKey) -> Option<&Value>;
    fn get_mut_value(&mut self, key: &Self::BorrowedKey) -> Option<&mut Value>;
    fn insert_value(&mut self, key: Self::Key, value: Value) -> Option<Value>;
    fn remove_value(&mut self, key: &Self::BorrowedKey) -> Option<Value>;
    fn is_empty_collection(&self) -> bool;
}

impl ValueCollection for Value {
    type BorrowedKey = ();
    type Key = ();

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
            Self::Object(m) => return Some(Self::Object(std::mem::take(m))),
            Self::Array(a) => return Some(Self::Array(std::mem::take(a))),
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
    type BorrowedKey = str;
    type Key = String;

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
    type BorrowedKey = isize;
    type Key = isize;

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
                    self.push(Value::Null);
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
        if let Some(index) = array_index(self, *key) {
            if index < self.len() {
                return Some(self.remove(index));
            }
        }

        None
    }

    fn is_empty_collection(&self) -> bool {
        self.is_empty()
    }
}

/// Returns the last coalesce key
pub fn skip_remaining_coalesce_segments<'a>(
    path_iter: &mut impl Iterator<Item = BorrowedSegment<'a>>,
) -> Cow<'a, str> {
    loop {
        match path_iter.next() {
            Some(BorrowedSegment::CoalesceField(_field)) => { /* skip */ }
            Some(BorrowedSegment::CoalesceEnd(field)) => return field,
            _ => unreachable!("malformed path. This is a bug"),
        }
    }
}

/// Returns the first matching coalesce key.
/// If non matches, returns Err with the last key.
pub fn get_matching_coalesce_key<'a>(
    initial_key: Cow<'a, str>,
    map: &BTreeMap<String, Value>,
    path_iter: &mut impl Iterator<Item = BorrowedSegment<'a>>,
) -> Result<Cow<'a, str>, Cow<'a, str>> {
    let mut key = initial_key;
    let mut coalesce_finished = false;
    let matched_key = loop {
        if map.get_value(key.as_ref()).is_some() {
            if !coalesce_finished {
                skip_remaining_coalesce_segments(path_iter);
            }

            break key;
        }

        if coalesce_finished {
            return Err(key);
        }

        match path_iter.next() {
            Some(BorrowedSegment::CoalesceField(field)) => {
                key = field;
            }
            Some(BorrowedSegment::CoalesceEnd(field)) => {
                key = field;
                coalesce_finished = true;
            }
            _ => unreachable!("malformed path. This is a bug"),
        }
    };

    Ok(matched_key)
}
