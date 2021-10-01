use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, PartialOrd, Debug, Clone, Deserialize, Serialize)]
pub enum Value {
    String(String),
    // Bytes(Bytes),
    Float(f64),
    Uint64(u64),
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

impl From<u8> for Value {
    fn from(u: u8) -> Self {
        Self::Uint64(u as u64)
    }
}

impl From<u64> for Value {
    fn from(u: u64) -> Self {
        Self::Uint64(u)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}