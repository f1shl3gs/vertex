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
        _key: impl AsRef<str>,
        _value: impl Into<Value> + Debug,
    ) -> Option<Value> {
        todo!()
    }
}