mod insert;
mod contains;
mod get;
mod path_iter;
mod remove;

use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::{Deserialize};
use bytes::Bytes;
use chrono::Utc;

use crate::{ByteSizeOf, Value};

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Deserialize)]
pub struct LogRecord {
    // time_unix_nano is the time when the event occurred
    pub time_unix_nano: u64,

    pub tags: BTreeMap<String, String>,

    pub fields: BTreeMap<String, Value>,
}

impl From<BTreeMap<String, Value>> for LogRecord {
    fn from(fields: BTreeMap<String, Value>) -> Self {
        Self {
            time_unix_nano: 0,
            tags: Default::default(),
            fields,
        }
    }
}

impl From<&str> for LogRecord {
    fn from(s: &str) -> Self {
        s.to_owned().into()
    }
}

impl From<String> for LogRecord {
    fn from(s: String) -> Self {
        Bytes::from(s).into()
    }
}

impl From<Bytes> for LogRecord {
    fn from(bs: Bytes) -> Self {
        let mut log = LogRecord::default();

        // TODO: log schema should be used here
        log.insert_field("message", bs);
        log.insert_field("timestamp", Utc::now());

        log
    }
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
        remove::remove(&mut self.fields, key.as_ref(), false)
    }

    pub fn remove_field_prune(
        &mut self,
        key: impl AsRef<str>,
    ) -> Option<Value> {
        remove::remove(&mut self.fields, key.as_ref(), true)
    }
}

#[cfg(test)]
pub fn fields_from_json(json_value: serde_json::Value) -> BTreeMap<String, Value> {
    match Value::from(json_value) {
        Value::Map(map) => map,
        sth => panic!("Expected a map, got {:?}", sth)
    }
}