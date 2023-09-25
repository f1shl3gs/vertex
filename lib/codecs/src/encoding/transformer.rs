use configurable::Configurable;
use event::log::Value;
use event::{Event, LogRecord, MaybeAsLogMut};
use lookup::{parse_path, path, OwnedPath};
use serde::{Deserialize, Deserializer, Serialize};

/// The format in which a timestamp should be represented.
#[derive(Configurable, Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TimestampFormat {
    /// Represent the timestamp as a Unix timestamp.
    Unix,

    /// Represent the timestamp as a RFC 3339 timestamp
    RFC3339,
}

/// Transformations to prepare an event for serialization.
#[derive(Configurable, Clone, Debug, Default, Serialize, PartialEq)]
pub struct Transformer {
    /// List of fields that will be included in the encoded event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    only_fields: Option<Vec<OwnedPath>>,

    /// List of fields that will be excluded from the encoded event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    except_fields: Option<Vec<String>>,

    /// Format used for timestamp fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timestamp_format: Option<TimestampFormat>,
}

impl<'de> Deserialize<'de> for Transformer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct TransformerInner {
            #[serde(default)]
            only_fields: Option<Vec<OwnedPath>>,
            #[serde(default)]
            except_fields: Option<Vec<String>>,
            #[serde(default)]
            timestamp_format: Option<TimestampFormat>,
        }

        let inner: TransformerInner = Deserialize::deserialize(deserializer)?;

        Self::new(
            inner.only_fields,
            inner.except_fields,
            inner.timestamp_format,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl Transformer {
    /// Create a new `Transformer`.
    ///
    /// Return `Err` if `only_fields` and `except_fields` fail validation, i.e. are not mutually
    /// exclusive.
    pub fn new(
        only_fields: Option<Vec<OwnedPath>>,
        except_fields: Option<Vec<String>>,
        timestamp_format: Option<TimestampFormat>,
    ) -> Result<Self, String> {
        // TODO: define the Error instead of using String
        Self::validate_fields(only_fields.as_ref(), except_fields.as_ref())?;

        Ok(Self {
            only_fields,
            except_fields,
            timestamp_format,
        })
    }

    /// Get the `Transformer's only_fields`.
    pub const fn only_fields(&self) -> &Option<Vec<OwnedPath>> {
        &self.only_fields
    }

    /// Get the `Transformer's except_fields`.
    pub const fn except_fields(&self) -> &Option<Vec<String>> {
        &self.except_fields
    }

    /// Get the `Transformer's timestamp_format`.
    pub const fn timestamp_format(&self) -> &Option<TimestampFormat> {
        &self.timestamp_format
    }

    /// Check if `except_fields` and `only_fields` items are mutually exclusive.
    ///
    /// If an error is returned, the entire encoding configuration should be considered
    /// inoperable.
    fn validate_fields(
        only_fields: Option<&Vec<OwnedPath>>,
        except_fields: Option<&Vec<String>>,
    ) -> Result<(), String> {
        if let (Some(only_fields), Some(except_fields)) = (only_fields, except_fields) {
            if except_fields.iter().any(|f| {
                let path_iter = parse_path(f);

                only_fields.iter().any(|v| v == &path_iter)
            }) {
                return Err(
                    "`except_fields` and `only_fields` should be mutually exclusive".into(),
                );
            }
        }

        Ok(())
    }

    /// Prepare an event for serialization by the given transformation rules.
    pub fn transform(&self, event: &mut Event) {
        // Rules are currently applied to logs only
        if let Some(log) = event.maybe_as_log_mut() {
            // Ordering in here should not matter.
            self.apply_except_fields(log);
            self.apply_only_fields(log);
            self.apply_timestamp_format(log);
        }
    }

    fn apply_only_fields(&self, log: &mut LogRecord) {
        if let Some(only_fields) = self.only_fields.as_ref() {
            let mut to_remove = match log.keys() {
                Some(keys) => keys
                    .filter(|field| {
                        let field_path = parse_path(field);

                        !only_fields
                            .iter()
                            .any(|only| field_path.segments.starts_with(&only.segments[..]))
                    })
                    .collect::<Vec<_>>(),
                None => vec![],
            };

            // reverse sort so that we delete array elements at the end first rather than
            // the start so that any `nulls` at the end are dropped and empty arrays are
            // pruned.
            to_remove.sort_by(|a, b| b.cmp(a));
            for removal in to_remove {
                log.remove_field_prune(removal.as_str(), true);
            }
        }
    }

    fn apply_except_fields(&self, log: &mut LogRecord) {
        if let Some(except_fields) = self.except_fields.as_ref() {
            for field in except_fields {
                log.remove_field(field.as_str());
            }
        }
    }

    fn apply_timestamp_format(&self, log: &mut LogRecord) {
        if let Some(timestamp_format) = self.timestamp_format.as_ref() {
            match timestamp_format {
                TimestampFormat::Unix => {
                    if log.fields.as_object().is_some() {
                        let mut timestamps = Vec::new();
                        for (k, v) in log.all_fields().expect("must be an object") {
                            if let Value::Timestamp(ts) = v {
                                timestamps.push((k.clone(), Value::Int64(ts.timestamp())));
                            }
                        }

                        for (k, v) in timestamps {
                            log.insert_field(k.as_str(), v);
                        }
                    } else {
                        // root is not an object
                        let timestamp = if let Value::Timestamp(ts) = log.fields {
                            Some(ts.timestamp())
                        } else {
                            None
                        };

                        if let Some(ts) = timestamp {
                            log.insert_field(path!(), Value::Int64(ts));
                        }
                    }
                }
                // RFC3339 is the default serialization of a timestamp
                TimestampFormat::RFC3339 => (),
            }
        }
    }

    /// Set the `except_fields` value.
    pub fn set_except_fields(&mut self, except_fields: Option<Vec<String>>) -> Result<(), String> {
        Self::validate_fields(self.only_fields.as_ref(), except_fields.as_ref())?;

        self.except_fields = except_fields;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use log_schema::log_schema;
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn serialize() {
        let string =
            r#"{"only_fields":["a.b[0]"],"except_fields":["ignore_me"],"timestamp_format":"unix"}"#;

        let transformer = serde_json::from_str::<Transformer>(string).unwrap();

        let serialized = serde_json::to_string(&transformer).unwrap();

        assert_eq!(string, serialized);
    }

    #[test]
    fn serialize_empty() {
        let string = "{}";

        let transformer = serde_json::from_str::<Transformer>(string).unwrap();

        let serialized = serde_json::to_string(&transformer).unwrap();

        assert_eq!(string, serialized);
    }

    #[test]
    fn deserialize_and_transform_except() {
        let transformer: Transformer =
            serde_yaml::from_str(r#"except_fields: ["a.b.c", "b", "c[0].y", "d\\.z", "e"]"#)
                .unwrap();
        let mut log = LogRecord::default();
        {
            log.insert_field("a", 1);
            log.insert_field("a.b", 1);
            log.insert_field("a.b.c", 1);
            log.insert_field("a.b.d", 1);
            log.insert_field("b[0]", 1);
            log.insert_field("b[1].x", 1);
            log.insert_field("c[0].x", 1);
            log.insert_field("c[0].y", 1);
            log.insert_field("d\\.z", 1);
            log.insert_field("e.a", 1);
            log.insert_field("e.b", 1);
        }
        let mut event = Event::from(log);
        transformer.transform(&mut event);
        assert!(!event.as_mut_log().contains("a.b.c"));
        assert!(!event.as_mut_log().contains("b"));
        assert!(!event.as_mut_log().contains("b[1].x"));
        assert!(!event.as_mut_log().contains("c[0].y"));
        assert!(!event.as_mut_log().contains("d\\.z"));
        assert!(!event.as_mut_log().contains("e.a"));

        assert!(event.as_mut_log().contains("a.b.d"));
        assert!(event.as_mut_log().contains("c[0].x"));
    }

    #[test]
    fn deserialize_and_transform_only() {
        let transformer: Transformer =
            serde_yaml::from_str(r#"only_fields: ["a.b.c", "b", "c[0].y", "g\\.z"]"#).unwrap();
        let mut log = LogRecord::default();
        {
            log.insert_field("a", 1);
            log.insert_field("a.b", 1);
            log.insert_field("a.b.c", 1);
            log.insert_field("a.b.d", 1);
            log.insert_field("b[0]", 1);
            log.insert_field("b[1].x", 1);
            log.insert_field("c[0].x", 1);
            log.insert_field("c[0].y", 1);
            log.insert_field("d.y", 1);
            log.insert_field("d.z", 1);
            log.insert_field("e[0]", 1);
            log.insert_field("e[1]", 1);
            log.insert_field("\"f.z\"", 1);
            log.insert_field("\"g.z\"", 1);
            log.insert_field("h", BTreeMap::new());
            log.insert_field("i", Vec::<Value>::new());
        }
        let mut event = Event::from(log);
        transformer.transform(&mut event);
        assert!(event.as_log().contains("a.b.c"));
        assert!(event.as_log().contains("b"));
        assert!(event.as_log().contains("b[1].x"));
        assert!(event.as_log().contains("c[0].y"));
        assert!(event.as_log().contains("\"g.z\""));

        assert!(!event.as_log().contains("a.b.d"));
        assert!(!event.as_log().contains("c[0].x"));
        assert!(!event.as_log().contains("d"));
        assert!(!event.as_log().contains("e"));
        assert!(!event.as_log().contains("f"));
        assert!(!event.as_log().contains("h"));
        assert!(!event.as_log().contains("i"));
    }

    #[test]
    fn deserialize_and_transform_timestamp() {
        let transformer: Transformer = serde_yaml::from_str(r#"timestamp_format: "unix""#).unwrap();
        let mut event = Event::Log(LogRecord::from("Demo"));
        let timestamp = event
            .as_mut_log()
            .get_field(log_schema().timestamp_key())
            .unwrap()
            .clone();
        let timestamp = timestamp.as_timestamp().unwrap();
        event
            .as_mut_log()
            .insert_field("another", Value::Timestamp(*timestamp));

        transformer.transform(&mut event);

        match event
            .as_mut_log()
            .get_field(log_schema().timestamp_key())
            .unwrap()
        {
            Value::Int64(_) => {}
            e => panic!(
                "Timestamp was not transformed into a Unix timestamp. Was {:?}",
                e
            ),
        }
        match event.as_mut_log().get_field("another").unwrap() {
            Value::Int64(_) => {}
            e => panic!(
                "Timestamp was not transformed into a Unix timestamp. Was {:?}",
                e
            ),
        }
    }

    #[test]
    fn exclusivity_violation() {
        let config: Result<Transformer, _> = serde_yaml::from_str(
            r#"
except_fields: ["Doop"]
only_fields: ["Doop"]
        "#,
        );
        assert!(config.is_err())
    }
}
