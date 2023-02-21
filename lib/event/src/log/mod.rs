pub mod value;

use std::collections::BTreeMap;
use std::fmt::Debug;

use bytes::Bytes;
use chrono::Utc;
use finalize::{BatchNotifier, EventFinalizer, EventFinalizers, Finalizable};
use log_schema::log_schema;
use lookup::Path;
use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};
use tracing::field::Field;
use value::keys::{all_fields, keys};
pub use value::Value;

use crate::metadata::EventMetadata;
use crate::tags::{skip_serializing_if_empty, Key, Tags};
use crate::{EventDataEq, MaybeAsLogMut};

/// The type alias for an array of `LogRecord` elements
pub type Logs = Vec<LogRecord>;

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct LogRecord {
    #[serde(skip_serializing_if = "skip_serializing_if_empty")]
    pub tags: Tags,

    pub fields: Value,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl From<BTreeMap<String, Value>> for LogRecord {
    fn from(fields: BTreeMap<String, Value>) -> Self {
        Self {
            tags: Default::default(),
            fields: Value::Object(fields),
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

        log.insert_field(log_schema().message_key(), bs);
        log.insert_field(log_schema().timestamp_key(), Utc::now());

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
    pub fn new(tags: Tags, fields: BTreeMap<String, Value>) -> Self {
        Self {
            tags,
            fields: fields.into(),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn into_parts(self) -> (Tags, Value, EventMetadata) {
        (self.tags, self.fields, self.metadata)
    }

    pub fn with_batch_notifier(mut self, batch: &BatchNotifier) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    #[inline]
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<crate::tags::Value>) {
        self.tags.insert(key, value)
    }

    #[inline]
    pub fn get_tag(&self, key: &Key) -> Option<&crate::tags::Value> {
        self.tags.get(key)
    }

    pub fn insert_field<'a>(
        &mut self,
        path: impl Path<'a>,
        value: impl Into<Value>,
    ) -> Option<Value> {
        self.fields.insert(path, value.into())
    }

    // deprecated
    pub fn try_insert_field<'a>(&mut self, key: impl Path<'a>, value: impl Into<Value> + Debug) {
        if !self.contains(key.clone()) {
            self.insert_field(key, value);
        }
    }

    pub fn contains<'a>(&self, path: impl Path<'a>) -> bool {
        self.fields.get(path).is_some()
    }

    pub fn get_field<'a>(&self, key: impl Path<'a>) -> Option<&Value> {
        self.fields.get(key)
    }

    pub fn get_field_mut<'a>(&mut self, key: impl Path<'a>) -> Option<&mut Value> {
        self.fields.get_mut(key)
    }

    pub fn remove_field<'a>(&mut self, key: impl Path<'a>) -> Option<Value> {
        self.fields.remove(key, false)
    }

    pub fn remove_field_prune<'a>(&mut self, key: impl Path<'a>, prune: bool) -> Option<Value> {
        self.fields.remove(key, prune)
    }

    pub fn keys(&self) -> Option<impl Iterator<Item = String> + '_> {
        match &self.fields {
            Value::Object(map) => Some(keys(map)),
            _ => None,
        }
    }

    pub fn all_fields(&self) -> Option<impl Iterator<Item = (String, &Value)> + Serialize> {
        self.as_map().map(all_fields)
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match &self.fields {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_map_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match &mut self.fields {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }

    pub fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }

    pub fn with_batch_notifier_option(mut self, batch: &Option<BatchNotifier>) -> Self {
        self.metadata = self.metadata.with_batch_notifier_option(batch);
        self
    }
}

#[cfg(test)]
pub fn fields_from_json(json_value: serde_json::Value) -> BTreeMap<String, Value> {
    match Value::from(json_value) {
        Value::Object(map) => map,
        sth => panic!("Expected a map, got {:?}", sth),
    }
}

#[macro_export]
macro_rules! fields {
    ( $($x:expr => $y:expr),* ) => ({
        let mut _map: std::collections::BTreeMap<String, $crate::log::Value> = std::collections::BTreeMap::new();
        $(
            _map.insert($x.into(), $y.into());
        )*
        _map
    });
    // Done with trailing comma
    ( $($x:expr => $y:expr,)* ) => (
        fields!{$($x => $y),*}
    );
    () => ({
        std::collections::BTreeMap<String, $crate::Value> = std::collections::BTreeMap::new();
    })
}
