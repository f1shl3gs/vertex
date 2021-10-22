mod insert;
mod contains;
mod get;
mod path_iter;
mod remove;

use serde::{Deserialize};
use crate::{ByteSizeOf, Value};
use std::collections::BTreeMap;
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct LogRecord {
    // time_unix_nano is the time when the event occurred
    pub time_unix_nano: u64,

    pub tags: BTreeMap<String, String>,

    pub fields: BTreeMap<String, Value>,
}

impl ByteSizeOf for LogRecord {
    fn allocated_bytes(&self) -> usize {
        self.tags.allocated_bytes() + self.fields.allocated_bytes()
    }
}

impl LogRecord {
    pub fn insert_field(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<Value> + Debug,
    ) -> Option<Value> {
        insert::insert(&mut self.fields, key.as_ref(), value.into())
    }

    pub fn try_insert_field(
        &mut self,
        key: impl AsRef<str>,
        value: impl Into<Value> + Debug,
    ) {
        let key = key.as_ref();
        if !self.contains(key) {
            self.insert_field(key, value);
        }
    }

    pub fn contains(&self, key: impl AsRef<str>) -> bool {
        contains::contains(&self.fields, key.as_ref())
    }

    pub fn get_field(
        &self,
        key: impl AsRef<str>,
    ) -> Option<&Value> {
        get::get(&self.fields, key.as_ref())
    }

    pub fn remove_field(
        &mut self,
        key: impl AsRef<str>,
    ) -> Option<Value> {
        remove::remove(&mut self.fields, key, false)
    }

    pub fn remove_field_prune(
        &mut self,
        key: impl AsRef<str>,
    ) -> Option<Value> {
        remove::remove(&mut self.fields, key, true)
    }
}

#[cfg(test)]
pub fn fields_from_json(json_value: serde_json::Value) -> BTreeMap<String, Value> {
    match Value::from(json_value) {
        Value::Map(map) => map,
        sth => panic!("Expected a map, got {:?}", sth)
    }
}