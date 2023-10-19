use bytes::Bytes;
use chrono::Utc;
use event::Event;
use log_schema::log_schema;
use smallvec::{smallvec, SmallVec};

use super::{DeserializeError, Deserializer};

/// Deserializer that builds `Event`s from a byte frame containing JSON
#[derive(Debug, Clone)]
pub struct JsonDeserializer;

impl JsonDeserializer {
    /// Creates a new `JsonDeserializer`
    pub const fn new() -> Self {
        Self
    }
}

impl Deserializer for JsonDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        // It's common to receive empty frames when parsing NDJSON, since it allows
        // multiple empty newlines. We proceed without a warning here.
        if buf.is_empty() {
            return Ok(smallvec![]);
        }

        let json: serde_json::Value = serde_json::from_slice(&buf)?;
        let mut events = match json {
            serde_json::Value::Array(array) => array
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<SmallVec<[Event; 1]>, _>>()?,
            _ => smallvec![json.try_into()?],
        };

        let timestamp = Utc::now();
        let timestamp_key = log_schema().timestamp_key();
        for event in &mut events {
            let log = event.as_mut_log();
            if !log.contains(timestamp_key) {
                log.insert(timestamp_key, timestamp);
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use event::event_path;

    use super::*;

    #[test]
    fn deserialize() {
        let input = Bytes::from(r#"{"foo":123}"#);
        let deserializer = JsonDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        {
            let event = events.next().unwrap();
            let log = event.as_log();
            assert_eq!(
                log.get_field(event_path!("foo")).unwrap().clone(),
                123.into()
            );
            assert!(log.get_field(log_schema().timestamp_key()).is_some())
        }

        assert_eq!(events.next(), None);
    }

    #[test]
    fn deserialize_empty() {
        let input = Bytes::from("");
        let deserializer = JsonDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn deserialize_invalid_json() {
        let input = Bytes::from(r#"{"foo"#);
        let deserializer = JsonDeserializer::new();

        assert!(deserializer.parse(input).is_err());
    }
}
