use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, VecDeque};

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use crate::trace::{AnyValue, Key, KeyValue};

/// A hash map with a capped number of attributes that retains
/// the most recently set entries.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct EvictedHashMap {
    map: HashMap<Key, AnyValue>,
    evict_list: VecDeque<Key>,
    max_len: u32,
    dropped_count: u32,
}

impl From<HashMap<Key, AnyValue>> for EvictedHashMap {
    fn from(map: HashMap<Key, AnyValue>) -> Self {
        let evict_list = map.keys().into_iter().cloned().collect();

        Self {
            map,
            evict_list,
            max_len: 128,
            dropped_count: 0,
        }
    }
}

impl<T> From<Vec<T>> for EvictedHashMap
where
    T: Into<(Key, AnyValue)>,
{
    fn from(kvs: Vec<T>) -> Self {
        kvs.into_iter().map(Into::into).collect()
    }
}

impl FromIterator<(Key, AnyValue)> for EvictedHashMap {
    fn from_iter<T: IntoIterator<Item = (Key, AnyValue)>>(iter: T) -> Self {
        iter.into_iter()
            .fold(EvictedHashMap::default(), |mut map, (key, value)| {
                map.insert(key, value);
                map
            })
    }
}

impl Default for EvictedHashMap {
    fn default() -> Self {
        Self::new(128, 0)
    }
}

impl PartialOrd for EvictedHashMap {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        todo!()
    }
}

impl ByteSizeOf for EvictedHashMap {
    fn allocated_bytes(&self) -> usize {
        // TODO
        0
    }
}

impl EvictedHashMap {
    /// Create a new `EvictedHashMap` with a given max length and capacity.
    pub fn new(max_len: u32, capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            evict_list: VecDeque::new(),
            max_len,
            dropped_count: 0,
        }
    }

    pub fn insert(&mut self, key: impl Into<Key>, value: impl Into<AnyValue>) {
        let key = key.into();
        let value = value.into();

        let mut already_exists = false;
        // Check for existing item
        match self.map.entry(key.clone()) {
            Entry::Occupied(mut occupied) => {
                occupied.insert(value);
                already_exists = true;
            }

            Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }

        if already_exists {
            self.move_key_to_front(key);
        } else {
            // Add new item
            self.evict_list.push_front(key);
        }

        // Verify size not exceeded
        #[allow(clippy::cast_possible_truncation)]
        if self.evict_list.len() as u32 > self.max_len {
            self.remove_oldest();
            self.dropped_count += 1;
        }
    }

    pub fn remove(&mut self, key: impl Into<Key>) -> Option<AnyValue> {
        let key = key.into();
        if let Some(value) = self.map.remove(&key) {
            self.move_key_to_front(key);
            self.evict_list.pop_front();
            Some(value)
        } else {
            None
        }
    }

    /// Inserts a key-value pair into the map.
    pub fn insert_key_value(&mut self, kv: KeyValue) {
        let KeyValue { key, value } = kv;
        self.insert(key, value);
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns the dropped attribute count
    pub fn dropped_count(&self) -> u32 {
        self.dropped_count
    }

    /// Returns a front-to-back iterator
    pub fn iter(&self) -> Iter<'_> {
        Iter(self.map.iter())
    }

    /// Returns a reference to the value corresponding to the key
    /// if it exists
    pub fn get(&self, key: &Key) -> Option<&AnyValue> {
        self.map.get(key)
    }

    pub fn contains_key(&self, key: impl Into<Key>) -> bool {
        self.map.contains_key(&(key.into()))
    }

    fn move_key_to_front(&mut self, key: Key) {
        if self.evict_list.is_empty() {
            // If empty, push front
            self.evict_list.push_front(key);
        } else if self.evict_list.front() == Some(&key) {
            // Already the front, ignore
        } else {
            // Else split linked lists around key and combine
            let key_idx = self
                .evict_list
                .iter()
                .position(|k| k == &key)
                .expect("key must exist in evicted hash map, this is a bug");

            let mut tail = self.evict_list.split_off(key_idx);
            let item = tail.pop_front().unwrap();
            self.evict_list.push_front(item);
            self.evict_list.append(&mut tail);
        }
    }

    fn remove_oldest(&mut self) {
        if let Some(oldest_item) = self.evict_list.pop_back() {
            self.map.remove(&oldest_item);
        }
    }
}

impl From<Vec<KeyValue>> for EvictedHashMap {
    fn from(kvs: Vec<KeyValue>) -> Self {
        let map = kvs
            .into_iter()
            .map(|kv| (kv.key, kv.value))
            .collect::<HashMap<Key, AnyValue>>();

        Self {
            map,
            evict_list: VecDeque::default(),
            max_len: 128,
            dropped_count: 0,
        }
    }
}

/// An owned iterator over the entries of a `EvictedHashMap`.
#[derive(Debug)]
pub struct IntoIter(std::collections::hash_map::IntoIter<Key, AnyValue>);

impl Iterator for IntoIter {
    type Item = (Key, AnyValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl IntoIterator for EvictedHashMap {
    type Item = (Key, AnyValue);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.map.into_iter())
    }
}

impl<'a> IntoIterator for &'a EvictedHashMap {
    type Item = (&'a Key, &'a AnyValue);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter(self.map.iter())
    }
}

/// An iterator over the entries of an `EvictedHashMap`.
#[derive(Debug)]
pub struct Iter<'a>(std::collections::hash_map::Iter<'a, Key, AnyValue>);

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a Key, &'a AnyValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn insert_over_capacity_test() {
        let max_len = 10;
        let mut map = EvictedHashMap::new(max_len, max_len as usize);

        for i in 0..=max_len {
            map.insert_key_value(Key::new(i.to_string()).bool(true));
        }

        assert_eq!(map.dropped_count, 1);
        assert_eq!(map.len(), max_len as usize);
        assert_eq!(
            map.map.keys().cloned().collect::<HashSet<_>>(),
            (1..=max_len)
                .map(|i| Key::new(i.to_string()))
                .collect::<HashSet<_>>()
        );
    }
}
