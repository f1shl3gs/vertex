use std::num::NonZeroUsize;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use configurable::{configurable_component, Configurable};
use event::log::path::parse_target_path;
use event::log::{OwnedTargetPath, Value};
use event::{Events, LogRecord};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use log_schema::log_schema;
use lru::LruCache;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Configurable, Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "lowercase")]
enum MatchType {
    /// The field names considered when deciding if an Event is a duplicate.
    /// This can also be globally set via the global log_schema options.
    /// Incompatible with the fields.ignore option.
    #[default]
    Match,

    /// The field names to ignore when deciding if an Event is a duplicate.
    /// Incompatible with the fields.match option.
    Ignore,
}

fn default_fields() -> Vec<OwnedTargetPath> {
    vec![
        log_schema().timestamp_key().clone(),
        log_schema().host_key().clone(),
        log_schema().message_key().clone(),
    ]
}

/// Deduplicates events to reduce data volume by eliminating copies of data.
#[configurable_component(transform, name = "dedup")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Options controlling how we cache recent Events for future duplicate checking.
    #[serde(default = "default_cache_config")]
    cache: NonZeroUsize,

    /// Options controlling what fields to match against.
    #[serde(default)]
    match_type: MatchType,

    #[serde(default = "default_fields")]
    fields: Vec<OwnedTargetPath>,
}

fn default_cache_config() -> NonZeroUsize {
    NonZeroUsize::new(4 * 1024).expect("static non-zero number")
}

#[async_trait]
#[typetag::serde(name = "dedup")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let cache = Arc::new(Mutex::new(LruCache::new(self.cache)));

        let dedup = Dedup {
            cache,
            fields: self.fields.clone(),
            match_type: self.match_type.clone(),
        };

        Ok(Transform::function(dedup))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

type TypeId = u8;

/// Assigns a unique number to each of the types supported by event::log::Value
const fn type_id_for_value(value: &Value) -> TypeId {
    match value {
        Value::Bytes(_) => 1,
        Value::Float(_) => 2,
        Value::Integer(_) => 3,
        Value::Boolean(_) => 4,
        Value::Array(_) => 5,
        Value::Object(_) => 6,
        Value::Timestamp(_) => 7,
        Value::Null => 8,
    }
}

/// A `CacheEntry` comes in two forms, depending on the FieldMatchConfig in use.
///
/// When matching fields, a CacheEntry contains a vector of optional 2-tuples.
/// Each element in the vector represents one field in the corresponding LogRecord.
/// Elements in the vector will correspond 1:1 (and in order) to the fields
/// specified in "fields.match". The tuples each store the TypeId for this field
/// and the data as Bytes for the field. There is no need to store the field
/// name because the elements of the vector correspond 1:1 to "fields.match",
/// so there is never any ambiguity about what field is being referred to. If
/// a field from "fields.match" does not show up in an incoming Event, the
/// CacheEntry will have None in the correspond location in the vector.
///
/// When ignoring fields, a CacheEntry contains a vector of 3-tuples. Each
/// element in the vector represents one field in the corresponding LogRecord.
/// The tuples will each contain the field name, TypeId, and data as Bytes for
/// the corresponding field (in that order). Since the set of fields that might
/// go into CacheEntries is not known at startup, we must store the field names
/// as part of CacheEntries. Since Event objects store their field in alphabetic
/// order (as they are backed by a BTreeMap), and we build CacheEntries by
/// iterating over the fields of the incoming Events, we know that the
/// CacheEntries for 2 equivalent events will always contain the fields in the
/// same order.
#[derive(Eq, Hash, PartialEq)]
enum CacheEntry {
    Match(Vec<Option<(TypeId, Bytes)>>),
    Ignore(Vec<(String, TypeId, Bytes)>),
}

#[derive(Clone)]
struct Dedup {
    cache: Arc<Mutex<LruCache<CacheEntry, bool>>>,
    match_type: MatchType,
    fields: Vec<OwnedTargetPath>,
}

impl Dedup {
    /// Takes in an Event array and returns a CacheEntry to place into the LRU cache
    /// containing all relevant information for the fields that need matching
    /// against according to the specified FieldMatchConfig.
    fn build_cache_entry(&self, log: &LogRecord) -> CacheEntry {
        match self.match_type {
            MatchType::Match => {
                let mut entry = Vec::new();

                for field_name in self.fields.iter() {
                    if let Some(value) = log.get(field_name) {
                        entry.push(Some((type_id_for_value(value), value.coerce_to_bytes())));
                    } else {
                        entry.push(None);
                    }
                }

                CacheEntry::Match(entry)
            }

            MatchType::Ignore => {
                let mut entry = Vec::new();

                if let Some(all_fields) = log.all_fields() {
                    for (name, value) in all_fields {
                        if let Ok(path) = parse_target_path(name.as_str()) {
                            if !self.fields.contains(&path) {
                                entry.push((
                                    name,
                                    type_id_for_value(value),
                                    value.coerce_to_bytes(),
                                ));
                            }
                        }
                    }
                }

                CacheEntry::Ignore(entry)
            }
        }
    }
}

impl FunctionTransform for Dedup {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        if let Events::Logs(logs) = events {
            let mut cache = self.cache.lock();

            let logs = logs
                .into_iter()
                .filter(|log| {
                    let entry = self.build_cache_entry(log);
                    cache.put(entry, true).is_none()
                })
                .collect::<Vec<_>>();

            output.push(logs)
        }
    }
}

#[cfg(test)]
mod tests {
    use event::{fields, Event};

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    fn make_match_transform(size: usize, fields: Vec<String>) -> Dedup {
        let cache = Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::try_from(size).unwrap(),
        )));

        let fields = fields
            .iter()
            .map(|f| parse_target_path(f).unwrap())
            .collect();

        Dedup {
            cache,
            fields,
            match_type: MatchType::Match,
        }
    }

    fn basic(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "some value",
            "unmatched" => "another value",
        ));

        // Test that unmatched field isn't considered
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "some value2",
            "unmatched" => "another value",
        ));

        // Test that matched field is considered
        let event3 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "some value",
            "unmatched" => "another value2",
        ));

        // First event should always be passed through as-is.
        let new_event = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(new_event, event1);

        // Second event differs in matched field so should be outputted even though it
        // has the same value for unmatched field.
        let new_event = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(new_event, event2);

        // Third event has the same value for "matched" as first event, so it should be dropped
        assert_eq!(None, transform_one(&mut transform, event3))
    }

    #[test]
    fn dedup_match_basic() {
        let transform = make_match_transform(5, vec!["matched".into()]);
        basic(transform);
    }

    fn make_ignore_transform(size: usize, given_fields: Vec<String>) -> Dedup {
        // "message" and "timestamp" are added automatically to all Events
        let mut fields = vec![
            parse_target_path("message").unwrap(),
            parse_target_path("timestamp").unwrap(),
        ];
        fields.extend(given_fields.iter().map(|f| parse_target_path(f).unwrap()));

        let cache = Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::try_from(size).unwrap(),
        )));

        Dedup {
            cache,
            fields,
            match_type: MatchType::Ignore,
        }
    }

    fn field_name_matters(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched1" => "some value"
        ));
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched2" => "some value",
        ));

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event has a different matched field name with the same value,
        // so it should not be considered a dupe.
        let got = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(got, event2);
    }

    #[test]
    fn dedup_ignore_field_name_matters() {
        let transform = make_ignore_transform(5, vec![]);
        field_name_matters(transform);
    }

    /// Test that two Events that are considered duplicates get handled that
    /// way, even if the order of the matched fields is different between the
    /// two.
    fn field_order_irrelevant(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched1" => "value1",
            "matched2" => "value2",
        ));

        // Add fields in opposite order
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched2" => "value2",
            "matched1" => "value1",
        ));

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event is the same just with different field order, so it
        // shouldn't be outputted.
        assert_eq!(None, transform_one(&mut transform, event2));
    }

    #[test]
    fn dedup_match_field_order_irrelevant() {
        let transform = make_ignore_transform(5, vec!["randomData".into()]);
        field_order_irrelevant(transform);
    }

    // Test the eviction behavior of the underlying LruCache
    fn age_out(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "some value",
        ));
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "some value2",
        ));

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event gets outputted because it's not a dupe. This cause the first
        // Event to be evicted from the cache.
        let got = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(got, event2);

        // Third event is a dupe but gets outputted anyway because the first
        // event has aged out of the cache.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);
    }

    #[test]
    fn dedup_match_age_out() {
        // Construct transform with a cache size of only 1 entry.
        let transform = make_match_transform(1, vec!["matched".into()]);
        age_out(transform);
    }

    // Test that two events with values for the matched fields that have different types
    // but the same string representation aren't considered duplicates.
    fn type_matching(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => "123",
        ));
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => 123
        ));

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event should also get passed through even though the string
        // representations of "matched" are the same.
        let got = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(got, event2);
    }

    #[test]
    fn dedup_match_type_matching() {
        let transform = make_match_transform(5, vec!["matched".into()]);
        type_matching(transform);
    }

    #[test]
    fn dedup_ignore_type_matching() {
        let transform = make_ignore_transform(5, vec![]);
        type_matching(transform);
    }

    // Test that two events where the matched field is a sub object and that object
    // contains values that have different types but the same string representation
    // aren't considered duplicates.
    fn type_matching_nested_objects(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => fields!(
                "key" => "123"
            )
        ));
        let event2 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => fields!(
                "key" => 123
            )
        ));

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event should also get passed through event through the string
        // representation of "matched" are the same.
        let got = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(got, event2)
    }

    #[test]
    fn dedup_match_type_matching_nested_objects() {
        let transform = make_match_transform(5, vec!["matched".into()]);
        type_matching_nested_objects(transform);
    }

    #[test]
    fn dedup_ignore_type_matching_nested_objects() {
        let transform = make_ignore_transform(5, vec![]);
        type_matching_nested_objects(transform);
    }

    fn ignore_vs_missing(mut transform: Dedup) {
        let event1 = Event::from(fields!(
            log_schema().message_key().to_string() => "message",
            "matched" => Value::Null,
        ));
        let event2 = Event::from("message");

        // First event should always be passed through as-is.
        let got = transform_one(&mut transform, event1.clone()).unwrap();
        assert_eq!(got, event1);

        // Second event should also get passed through as null is different than
        // missing
        let got = transform_one(&mut transform, event2.clone()).unwrap();
        assert_eq!(got, event2);
    }

    #[test]
    fn dedup_match_null_vs_missing() {
        let transform = make_match_transform(5, vec!["matched".into()]);
        ignore_vs_missing(transform);
    }

    #[test]
    fn dedup_ignore_null_vs_missing() {
        let transform = make_ignore_transform(5, vec![]);
        ignore_vs_missing(transform);
    }
}
