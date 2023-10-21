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
use value::path::{PathPrefix, ValuePath};
pub use value::{
    event_path, metadata_path, owned_value_path, parse_value_path, path, path::TargetPath,
    OwnedTargetPath, OwnedValuePath, Value,
};

use crate::log::keys::{all_fields, keys};
use crate::metadata::EventMetadata;
use crate::{
    BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, Finalizable, MaybeAsLogMut,
};

/// The type alias for an array of `LogRecord` elements
pub type Logs = Vec<LogRecord>;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LogRecord {
    #[serde(skip)]
    metadata: EventMetadata,

    #[serde(flatten)]
    fields: Value,
}

impl Default for LogRecord {
    fn default() -> Self {
        Self {
            fields: Value::Object(BTreeMap::new()),
            metadata: EventMetadata::default(),
        }
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
        self.metadata.allocated_bytes() + self.fields.allocated_bytes()
    }
}

impl Finalizable for LogRecord {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.metadata.take_finalizers()
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

impl From<BTreeMap<String, Value>> for LogRecord {
    fn from(fields: BTreeMap<String, Value>) -> Self {
        Self {
            fields: Value::Object(fields),
            metadata: EventMetadata::default(),
        }
    }
}

impl From<Value> for LogRecord {
    fn from(value: Value) -> Self {
        Self {
            metadata: EventMetadata::default(),
            fields: value,
        }
    }
}

impl LogRecord {
    #[inline]
    pub fn into_parts(self) -> (EventMetadata, Value) {
        (self.metadata, self.fields)
    }

    #[inline]
    pub fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    #[inline]
    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }

    /// This is added to the "event metadata", nested under the name "vertex".
    pub fn insert_metadata<'a>(&mut self, key: impl ValuePath<'a>, value: impl Into<Value>) {
        self.metadata
            .value
            .insert(path!("vertex").concat(key), value);
    }

    #[inline]
    pub fn value(&self) -> &Value {
        &self.fields
    }

    #[inline]
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
    pub fn get<'a>(&self, path: impl TargetPath<'a>) -> Option<&Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.get(path.value_path()),
            PathPrefix::Metadata => self.metadata.value.get(path.value_path()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn get_mut<'a>(&mut self, path: impl TargetPath<'a>) -> Option<&mut Value> {
        match path.prefix() {
            PathPrefix::Event => self.fields.get_mut(path.value_path()),
            PathPrefix::Metadata => self.metadata.value.get_mut(path.value_path()),
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn remove<'a>(&mut self, path: impl TargetPath<'a>) -> Option<Value> {
        self.remove_prune(path, false)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn remove_prune<'a>(&mut self, path: impl TargetPath<'a>, prune: bool) -> Option<Value> {
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

    #[must_use]
    pub fn with_batch_notifier(mut self, batch: &BatchNotifier) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
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
