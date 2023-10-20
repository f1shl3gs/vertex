mod all_fields;
mod keys;
mod tracing;

use std::collections::BTreeMap;
use std::fmt::Debug;

use bytes::Bytes;
use chrono::Utc;
use log_schema::log_schema;
use measurable::ByteSizeOf;
use serde::Serialize;
use value::path::{PathPrefix, TargetPath, ValuePath};
pub use value::{
    event_path, metadata_path, parse_value_path, path, OwnedTargetPath, OwnedValuePath, Value,
};

use crate::log::keys::{all_fields, keys};
use crate::metadata::EventMetadata;
use crate::tags::{skip_serializing_if_empty, Key, Tags};
use crate::{
    BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, Finalizable, MaybeAsLogMut,
};

/// The type alias for an array of `LogRecord` elements
pub type Logs = Vec<LogRecord>;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LogRecord {
    #[serde(skip_serializing_if = "skip_serializing_if_empty")]
    pub tags: Tags,

    #[serde(flatten)]
    pub fields: Value,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl Default for LogRecord {
    fn default() -> Self {
        Self {
            tags: Tags::default(),
            fields: Value::Object(BTreeMap::new()),
            metadata: EventMetadata::default(),
        }
    }
}

impl From<BTreeMap<String, Value>> for LogRecord {
    fn from(fields: BTreeMap<String, Value>) -> Self {
        Self {
            tags: Tags::default(),
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

        log.insert(log_schema().message_key(), bs);
        log.insert(log_schema().timestamp_key(), Utc::now());

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
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn into_parts(self) -> (Tags, Value, EventMetadata) {
        (self.tags, self.fields, self.metadata)
    }

    #[must_use]
    pub fn with_batch_notifier(mut self, batch: &BatchNotifier) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    #[inline]
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<crate::tags::Value>) {
        self.tags.insert(key, value);
    }

    #[inline]
    pub fn get_tag(&self, key: &Key) -> Option<&crate::tags::Value> {
        self.tags.get(key)
    }

    pub fn value(&self) -> &Value {
        &self.fields
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.fields
    }

    #[allow(clippy::needless_pass_by_value)] // TargetPath is always a reference
    pub fn insert<'a>(
        &mut self,
        path: impl TargetPath<'a>,
        value: impl Into<Value>,
    ) -> Option<Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.insert(path.value_path(), value.into()),
            PathPrefix::Metadata => self.metadata.value.insert(path.value_path(), value.into()),
        }
    }

    // deprecated
    pub fn try_insert<'a>(&mut self, key: impl TargetPath<'a>, value: impl Into<Value> + Debug) {
        if !self.contains(key.clone()) {
            self.insert(key, value);
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn contains<'a>(&self, path: impl TargetPath<'a>) -> bool {
        match path.prefix() {
            PathPrefix::Event => self.fields.contains(path.value_path()),
            PathPrefix::Metadata => self.metadata.value.contains(path.value_path()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn get_field<'a>(&self, path: impl TargetPath<'a>) -> Option<&Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.get(path.value_path()),
            PathPrefix::Metadata => self.metadata.value.get(path.value_path()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn get_field_mut<'a>(&mut self, path: impl TargetPath<'a>) -> Option<&mut Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.get_mut(path.value_path()),
            PathPrefix::Metadata => self.metadata.value.get_mut(path.value_path()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn remove_field<'a>(&mut self, path: impl TargetPath<'a>) -> Option<Value> {
        self.remove_field_prune(path, false)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn remove_field_prune<'a>(
        &mut self,
        path: impl TargetPath<'a>,
        prune: bool,
    ) -> Option<Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.remove(path.value_path(), prune),
            PathPrefix::Metadata => self.metadata.value.remove(path.value_path(), prune),
        }
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

    #[must_use]
    pub fn with_batch_notifier_option(mut self, batch: &Option<BatchNotifier>) -> Self {
        self.metadata = self.metadata.with_batch_notifier_option(batch);
        self
    }

    /// This is added to "event metadata", nested under the source name.
    pub fn insert_source_metadata<'a>(
        &mut self,
        source_name: &'a str,
        key: impl ValuePath<'a>,
        value: impl Into<Value>,
    ) {
        self.metadata
            .value
            .insert(path!(source_name).concat(key), value);
    }
}

#[cfg(test)]
#[allow(clippy::missing_panics_doc)]
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
