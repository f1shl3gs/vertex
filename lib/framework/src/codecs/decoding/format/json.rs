use bytes::Bytes;
use chrono::Utc;
use event::Event;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use super::Deserializer;

/// config used to build a `JsonDeserializer`
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct JsonDeserializerConfig;

impl JsonDeserializerConfig {
    /// Build the `JsonDeserializer` from this configuration
    pub fn build(&self) -> JsonDeserializer {
        JsonDeserializer
    }
}

/// Deserializer that builds `Event`s from a byte frame containing JSON
#[derive(Clone, Debug, Default)]
pub struct JsonDeserializer;

impl JsonDeserializer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Deserializer for JsonDeserializer {
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>> {
        // It's common to receive empty frames when parsing NDJSON, since it
        // allows multiple empty newlines. We proceed without a waring here
        if bytes.is_empty() {
            return Ok(smallvec![]);
        }

        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|err| format!("Error parsing JSON: {:?}", err))?;

        let mut events = match json {
            serde_json::Value::Array(values) => values
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<SmallVec<[Event; 1]>, _>>()?,
            _ => smallvec![json.try_into()?],
        };

        let timestamp = Utc::now();

        for event in &mut events {
            let log = event.as_mut_log();
            let timestamp_key = log_schema().timestamp_key();

            if !log.contains(timestamp_key) {
                log.insert_field(timestamp_key, timestamp);
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::log::Value;

    #[test]
    fn deserialize_json() {
        let input = Bytes::from(r#"{ "foo": 123 }"#);
        let deserializer = JsonDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        {
            let event = events.next().unwrap();
            let log = event.as_log();
            assert_eq!(log.get_field("foo").unwrap(), &Value::from(123));
            assert!(log.get_field(log_schema().timestamp_key()).is_some());
        }

        assert_eq!(events.next(), None);
    }

    #[test]
    fn deserialize_json_array() {
        let input = Bytes::from(r#"[{ "foo": 123 }, { "bar": 456 }]"#);
        let deserializer = JsonDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        {
            let event = events.next().unwrap();
            let log = event.as_log();
            assert_eq!(log.get_field("foo").unwrap(), &Value::from(123));
            assert!(log.get_field(log_schema().timestamp_key()).is_some());
        }

        {
            let event = events.next().unwrap();
            let log = event.as_log();
            assert_eq!(log.get_field("bar").unwrap(), &Value::from(456));
            assert!(log.get_field(log_schema().timestamp_key()).is_some());
        }

        assert_eq!(events.next(), None);
    }

    #[test]
    fn deserialize_skip_empty() {
        let input = Bytes::from("");
        let deserializer = JsonDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        assert_eq!(events.next(), None);
    }

    #[test]
    fn deserialize_error_invalid_json() {
        let input = Bytes::from("{ foo");
        let deserializer = JsonDeserializer::new();

        assert!(deserializer.parse(input).is_err());
    }
}
