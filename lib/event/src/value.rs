use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum Value {
    String(String),
    // Bytes(Bytes),
    Float(f64),
    Map(BTreeMap<String, Value>),
}

/*
impl From<Bytes> for Value {
    fn from(bytes: Bytes) -> Self {
        Value::Bytes(bytes)
    }
}
*/
impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<BTreeMap<String, Value>> for Value {
    fn from(m: BTreeMap<String, Value>) -> Self {
        Value::Map(m)
    }
}