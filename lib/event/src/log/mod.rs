mod contains;
mod get;
mod insert;
mod keys;
pub mod path_iter;
mod remove;
pub mod value;

use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

use bytes::Bytes;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;
use tracing::field::Field;

use crate::encoding::MaybeAsLogMut;
use crate::log::keys::all_fields;
use crate::metadata::EventMetadata;
use crate::{EventDataEq, EventFinalizer, EventFinalizers, Finalizable, Value};

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct LogRecord {
    pub tags: BTreeMap<String, String>,

    pub fields: BTreeMap<String, Value>,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl From<BTreeMap<String, Value>> for LogRecord {
    fn from(fields: BTreeMap<String, Value>) -> Self {
        Self {
            tags: Default::default(),
            fields,
            metadata: EventMetadata::default(),
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

impl EventDataEq for LogRecord {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

impl MaybeAsLogMut for LogRecord {
    fn maybe_as_log_mut(&mut self) -> Option<&mut LogRecord> {
        Some(self)
    }
}

impl tracing::field::Visit for LogRecord {
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert_field(field.name(), value);
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert_field(field.name(), value);
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert_field(field.name(), value);
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert_field(field.name(), value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.insert_field(field.name(), format!("{:?}", value));
    }
}

impl From<&tracing::Event<'_>> for LogRecord {
    fn from(event: &tracing::Event<'_>) -> Self {
        let now = chrono::Utc::now();
        let mut log = LogRecord::default();
        event.record(&mut log);

        log.insert_field("timestamp", now);

        let meta = event.metadata();
        log.insert_field("metadata.level", meta.level().to_string());
        log.insert_field("metadata.target", meta.target().to_string());
        log.insert_field(
            "metadata.module_path",
            meta.module_path()
                .map_or(Value::Null, |mp| Value::Bytes(mp.to_string().into())),
        );
        log.insert_field(
            "metadata.kind",
            if meta.is_event() {
                Value::Bytes("event".to_string().into())
            } else if meta.is_span() {
                Value::Bytes("span".to_string().into())
            } else {
                Value::Null
            },
        );

        log
    }
}

impl ByteSizeOf for LogRecord {
    fn allocated_bytes(&self) -> usize {
        self.tags.allocated_bytes() + self.fields.allocated_bytes()
    }
}

impl Finalizable for LogRecord {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.metadata.take_finalizers()
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

    pub fn try_insert_field(&mut self, key: impl AsRef<str>, value: impl Into<Value> + Debug) {
        let key = key.as_ref();
        if !self.contains(key) {
            self.insert_field(key, value);
        }
    }

    /// This function will insert a key in place without reference to any pathing
    /// information in the key. It will insert over the top of any value that
    /// exists in the map already.
    pub fn insert_flat_field<K, V>(&mut self, key: K, value: V) -> Option<Value>
    where
        K: Into<String> + Display,
        V: Into<Value> + Debug,
    {
        self.fields.insert(key.into(), value.into())
    }

    pub fn contains(&self, key: impl AsRef<str>) -> bool {
        contains::contains(&self.fields, key.as_ref())
    }

    pub fn get_field(&self, key: impl AsRef<str>) -> Option<&Value> {
        get::get(&self.fields, key.as_ref())
    }

    pub fn remove_field(&mut self, key: impl AsRef<str>) -> Option<Value> {
        remove::remove(&mut self.fields, key.as_ref(), false)
    }

    pub fn remove_field_prune(&mut self, key: impl AsRef<str>, prune: bool) -> Option<Value> {
        remove::remove(&mut self.fields, key.as_ref(), prune)
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        keys::keys(&self.fields)
    }

    pub fn all_fields(&self) -> impl Iterator<Item = (String, &Value)> + Serialize {
        all_fields(&self.fields)
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }

    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }
}

#[cfg(test)]
pub fn fields_from_json(json_value: serde_json::Value) -> BTreeMap<String, Value> {
    match Value::from(json_value) {
        Value::Map(map) => map,
        sth => panic!("Expected a map, got {:?}", sth),
    }
}
