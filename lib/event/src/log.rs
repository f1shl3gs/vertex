use serde::{Deserialize};

use crate::{ByteSizeOf, Value};
use std::collections::BTreeMap;

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