use serde::{Deserialize};

use crate::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize)]
pub struct LogRecord {
    // time_unix_nano is the time when the event occurred
    pub time_unix_nano: u64,

    pub tags: BTreeMap<String, String>,

    pub fields: BTreeMap<String, Value>,
}