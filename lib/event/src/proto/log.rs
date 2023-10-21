use std::collections::BTreeMap;

use chrono::TimeZone;
use tracing::error;

use crate::log::Value;
use crate::metadata::WithMetadata;
use crate::proto::{
    proto_event::Log as PLog, value::Kind as PVKind, Value as PValue, ValueArray as PValueArray,
    ValueMap as PValueMap,
};
use crate::LogRecord;

fn encode_array(items: Vec<Value>) -> PValueArray {
    PValueArray {
        items: items.into_iter().map(encode_value).collect(),
    }
}

fn encode_map(fields: BTreeMap<String, Value>) -> PValueMap {
    PValueMap {
        map: fields
            .into_iter()
            .map(|(k, v)| (k, encode_value(v)))
            .collect(),
    }
}

pub(super) fn encode_value(value: Value) -> PValue {
    PValue {
        kind: match value {
            Value::Bytes(bytes) => Some(PVKind::Bytes(bytes)),
            Value::Float(f) => Some(PVKind::Float(f)),
            Value::Integer(i) => Some(PVKind::Integer(i)),
            Value::Boolean(b) => Some(PVKind::Boolean(b)),
            Value::Array(arr) => Some(PVKind::Array(encode_array(arr))),
            Value::Object(m) => Some(PVKind::Map(encode_map(m))),
            Value::Timestamp(ts) => Some(PVKind::Timestamp(prost_types::Timestamp {
                seconds: ts.timestamp(),
                nanos: ts.timestamp_subsec_nanos() as i32,
            })),
            Value::Null => Some(PVKind::Null(0)),
        },
    }
}

pub(super) fn decode_value(input: PValue) -> Option<Value> {
    match input.kind {
        Some(PVKind::Bytes(b)) => Some(Value::Bytes(b)),
        Some(PVKind::Float(f)) => Some(Value::Float(f)),
        Some(PVKind::Integer(i)) => Some(Value::Integer(i)),
        Some(PVKind::Boolean(b)) => Some(Value::Boolean(b)),
        Some(PVKind::Array(a)) => decode_array(a.items),
        Some(PVKind::Map(m)) => decode_map(m.map),
        Some(PVKind::Timestamp(ts)) => Some(Value::Timestamp(
            chrono::Utc.timestamp_nanos(ts.seconds * 1_000_000_000 + ts.nanos as i64),
        )),
        Some(PVKind::Null(_)) => Some(Value::Null),
        None => {
            error!(message = "Encode event contains unknown value kind.");
            None
        }
    }
}

fn decode_map(fields: BTreeMap<String, PValue>) -> Option<Value> {
    let mut accum: BTreeMap<String, Value> = BTreeMap::new();

    for (key, value) in fields {
        match decode_value(value) {
            Some(value) => {
                accum.insert(key, value);
            }
            None => return None,
        }
    }

    Some(Value::Object(accum))
}

fn decode_array(items: Vec<PValue>) -> Option<Value> {
    let mut accum = Vec::with_capacity(items.len());

    for value in items {
        match decode_value(value) {
            Some(value) => accum.push(value),
            None => return None,
        }
    }

    Some(Value::Array(accum))
}

impl From<PLog> for LogRecord {
    fn from(log: PLog) -> Self {
        let fields = log
            .fields
            .map(|f| decode_value(f).unwrap_or_else(|| Value::Object(BTreeMap::new())))
            .unwrap_or_else(|| Value::Object(BTreeMap::new()));

        LogRecord::from(fields)
    }
}

impl From<LogRecord> for PLog {
    fn from(value: LogRecord) -> Self {
        let (metadata, fields) = value.into_parts();

        PLog {
            metadata: Some(metadata.into()),
            fields: Some(encode_value(fields)),
        }
    }
}
