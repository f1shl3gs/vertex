use bytes::Bytes;
use chrono::Utc;
use configurable::Configurable;
use event::Event;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use super::{DeserializeError, Deserializer};
use crate::serde::{default_lossy, skip_serializing_if_default};

/// Config used to build a `JsonDeserializer`
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct JsonDeserializerConfig {
    /// Determines whether or not to replace invalid UTF-8 sequences instead of failing.
    ///
    /// When true, invalid UTF-8 sequences are replaced with the [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD].
    ///
    /// [U+FFFD]: https://en.wikipedia.org/wiki/Specials_(Unicode_block)#Replacement_character
    #[serde(
        default = "default_lossy",
        skip_serializing_if = "skip_serializing_if_default"
    )]
    lossy: bool,
}

impl JsonDeserializerConfig {
    /// Build the `JsonDeserializer` from this configuration.
    #[inline]
    pub fn build(&self) -> JsonDeserializer {
        JsonDeserializer { lossy: self.lossy }
    }
}

/// Deserializer that builds `Event`s from a byte frame containing JSON
#[derive(Clone, Debug)]
pub struct JsonDeserializer {
    lossy: bool,
}

impl JsonDeserializer {
    /// Creates a new `JsonDeserializer`
    pub const fn new(lossy: bool) -> Self {
        Self { lossy }
    }
}

impl Deserializer for JsonDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        // It's common to receive empty frames when parsing NDJSON, since it allows
        // multiple empty newlines. We proceed without a warning here.
        if buf.is_empty() {
            return Ok(smallvec![]);
        }

        let json: serde_json::Value = if self.lossy {
            serde_json::from_str(&String::from_utf8_lossy(&buf))
        } else {
            serde_json::from_slice(&buf)
        }?;
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
        let deserializer = JsonDeserializer::new(true);

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        {
            let event = events.next().unwrap();
            let log = event.as_log();
            assert_eq!(log.get(event_path!("foo")).unwrap().clone(), 123.into());
            assert!(log.get(log_schema().timestamp_key()).is_some())
        }

        assert_eq!(events.next(), None);
    }

    #[test]
    fn deserialize_empty() {
        let input = Bytes::from("");
        let deserializer = JsonDeserializer::new(true);

        let events = deserializer.parse(input).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn deserialize_invalid_json() {
        let input = Bytes::from(r#"{"foo"#);
        let deserializer = JsonDeserializer::new(true);

        assert!(deserializer.parse(input).is_err());
    }
}
