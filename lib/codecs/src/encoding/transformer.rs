use std::collections::BTreeMap;
use std::fmt::Formatter;

use configurable::Configurable;
use event::log::path::{parse_target_path, PathPrefix};
use event::log::{OwnedValuePath, Value};
use event::{event_path, Event, LogRecord, MaybeAsLogMut};
use serde::de::MapAccess;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
#[derive(Configurable, Clone, Debug, Default, PartialEq)]
pub struct Transformer {
    /// List of fields that will be included in the encoded event.
    only_fields: Option<Vec<OwnedValuePath>>,

    /// List of fields that will be excluded from the encoded event.
    except_fields: Option<Vec<OwnedValuePath>>,

    /// Format used for timestamp fields.
    timestamp_format: Option<TimestampFormat>,
}

impl<'de> Deserialize<'de> for Transformer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        const EXPECTED_FIELDS: [&str; 3] = ["only_fields", "except_fields", "timestamp_format"];
        struct TransformerVisitor;

        impl<'de> serde::de::Visitor<'de> for TransformerVisitor {
            type Value = Transformer;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("expect a map")
            }

            #[allow(unused_mut)]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut only_fields = None;
                let mut except_fields = None;
                let mut timestamp_format = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "except_fields" => except_fields = map.next_value()?,
                        "only_fields" => only_fields = map.next_value()?,
                        "timestamp_format" => timestamp_format = map.next_value()?,
                        _ => return Err(serde::de::Error::unknown_field(key, &EXPECTED_FIELDS)),
                    }
                }

                Transformer::new(only_fields, except_fields, timestamp_format)
                    .map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_struct("Transformer", &EXPECTED_FIELDS, TransformerVisitor)
    }
}

impl Serialize for Transformer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Transformer", 3)?;
        if let Some(except_fields) = &self.except_fields {
            s.serialize_field("except_fields", except_fields)?;
        }

        if let Some(only_fields) = &self.only_fields {
            s.serialize_field("only_fields", only_fields)?;
        }

        if let Some(timestamp_format) = &self.timestamp_format {
            s.serialize_field("timestamp_format", timestamp_format)?;
        }

        s.end()
    }
}

impl Transformer {
    /// Create a new `Transformer`.
    ///
    /// Return `Err` if `only_fields` and `except_fields` fail validation, i.e. are not mutually
    /// exclusive.
    pub fn new(
        only_fields: Option<Vec<OwnedValuePath>>,
        except_fields: Option<Vec<OwnedValuePath>>,
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
    pub const fn only_fields(&self) -> &Option<Vec<OwnedValuePath>> {
        &self.only_fields
    }

    /// Get the `Transformer's except_fields`.
    pub const fn except_fields(&self) -> &Option<Vec<OwnedValuePath>> {
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
        only_fields: Option<&Vec<OwnedValuePath>>,
        except_fields: Option<&Vec<OwnedValuePath>>,
    ) -> Result<(), String> {
        if let (Some(only_fields), Some(except_fields)) = (only_fields, except_fields) {
            if except_fields
                .iter()
                .any(|f| only_fields.iter().any(|v| v == f))
            {
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
            let mut old_value = std::mem::replace(log.value_mut(), Value::Object(BTreeMap::new()));

            for field in only_fields {
                if let Some(value) = old_value.remove(field, true) {
                    log.insert((PathPrefix::Event, field), value);
                }
            }
        }
    }

    fn apply_except_fields(&self, log: &mut LogRecord) {
        if let Some(except_fields) = self.except_fields.as_ref() {
            for field in except_fields {
                log.remove((PathPrefix::Event, field));
            }
        }
    }

    fn apply_timestamp_format(&self, log: &mut LogRecord) {
        if let Some(timestamp_format) = self.timestamp_format.as_ref() {
            match timestamp_format {
                TimestampFormat::Unix => {
                    if log.value().as_object().is_some() {
                        let mut timestamps = Vec::new();
                        for (k, v) in log.all_fields().expect("must be an object") {
                            if let Value::Timestamp(ts) = v {
                                timestamps.push((k.clone(), Value::Integer(ts.timestamp())));
                            }
                        }

                        for (k, v) in timestamps {
                            let path = parse_target_path(k.as_ref()).expect("path should be valid");
                            log.insert(&path, v);
                        }
                    } else {
                        // root is not an object
                        let timestamp = if let Value::Timestamp(ts) = log.value() {
                            Some(ts.timestamp())
                        } else {
                            None
                        };

                        if let Some(ts) = timestamp {
                            log.insert(event_path!(), Value::Integer(ts));
                        }
                    }
                }
                // RFC3339 is the default serialization of a timestamp
                TimestampFormat::RFC3339 => (),
            }
        }
    }

    /// Set the `except_fields` value.
    pub fn set_except_fields(
        &mut self,
        except_fields: Option<Vec<OwnedValuePath>>,
    ) -> Result<(), String> {
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
            r#"{"except_fields":["ignore_me"],"only_fields":["a.b[0]"],"timestamp_format":"unix"}"#;

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
            serde_yaml::from_str(r#"except_fields: ["a.b.c", "b", "c[0].y", "d.z", "e"]"#).unwrap();
        let mut log = LogRecord::default();
        {
            log.insert("a", 1);
            log.insert("a.b", 1);
            log.insert("a.b.c", 1);
            log.insert("a.b.d", 1);
            log.insert("b[0]", 1);
            log.insert("b[1].x", 1);
            log.insert("c[0].x", 1);
            log.insert("c[0].y", 1);
            log.insert("d.z", 1);
            log.insert("e.a", 1);
            log.insert("e.b", 1);
        }
        let mut event = Event::from(log);
        transformer.transform(&mut event);
        assert!(!event.as_mut_log().contains("a.b.c"));
        assert!(!event.as_mut_log().contains("b"));
        assert!(!event.as_mut_log().contains("b[1].x"));
        assert!(!event.as_mut_log().contains("c[0].y"));
        assert!(!event.as_mut_log().contains("d.z"));
        assert!(!event.as_mut_log().contains("e.a"));

        assert!(event.as_mut_log().contains("a.b.d"));
        assert!(event.as_mut_log().contains("c[0].x"));
    }

    #[test]
    fn deserialize_and_transform_only() {
        let transformer: Transformer =
            serde_yaml::from_str(r#"only_fields: ["a.b.c", "b", "c[0].y", "\"g.z\""]"#).unwrap();
        let mut log = LogRecord::default();
        {
            log.insert("a", 1);
            log.insert("a.b", 1);
            log.insert("a.b.c", 1);
            log.insert("a.b.d", 1);
            log.insert("b[0]", 1);
            log.insert("b[1].x", 1);
            log.insert("c[0].x", 1);
            log.insert("c[0].y", 1);
            log.insert("d.y", 1);
            log.insert("d.z", 1);
            log.insert("e[0]", 1);
            log.insert("e[1]", 1);
            log.insert("\"f.z\"", 1);
            log.insert("\"g.z\"", 1);
            log.insert("h", BTreeMap::new());
            log.insert("i", Vec::<Value>::new());
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
            .get(log_schema().timestamp_key())
            .unwrap()
            .clone();
        let timestamp = match timestamp {
            Value::Timestamp(ts) => ts,
            _ => unreachable!(),
        };
        event
            .as_mut_log()
            .insert("another", Value::Timestamp(timestamp));

        transformer.transform(&mut event);

        match event
            .as_mut_log()
            .get(log_schema().timestamp_key())
            .unwrap()
        {
            Value::Integer(_) => {}
            e => panic!(
                "Timestamp was not transformed into a Unix timestamp. Was {:?}",
                e
            ),
        }
        match event.as_mut_log().get("another").unwrap() {
            Value::Integer(_) => {}
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
