use bytes::Bytes;
use event::{Event, LogRecord};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use super::Deserializer;

/// Config used to build a `BytesDeserializer`
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BytesDeserializerConfig;

impl BytesDeserializerConfig {
    /// Creates a new `BytesDeserializerConfig`
    pub const fn new() -> Self {
        Self
    }

    pub fn build(&self) -> BytesDeserializer {
        BytesDeserializer::new()
    }
}

/// Deserializer that converts bytes to an `Event`
#[derive(Debug, Clone)]
pub struct BytesDeserializer {
    log_schema_message_key: &'static str,
}

impl Default for BytesDeserializer {
    fn default() -> Self {
        Self::new()
    }
}

impl BytesDeserializer {
    /// Creates a new `BytesDeserializer`
    pub fn new() -> Self {
        Self {
            log_schema_message_key: log_schema().message_key(),
        }
    }
}

impl Deserializer for BytesDeserializer {
    fn parse(&self, bytes: Bytes) -> crate::Result<SmallVec<[Event; 1]>> {
        let mut log = LogRecord::default();
        log.insert_field(self.log_schema_message_key, bytes);
        Ok(smallvec![log.into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::log::Value;

    #[test]
    fn parse_bytes() {
        let input = Bytes::from("foo");
        let deserializer = BytesDeserializer::new();

        let events = deserializer.parse(input).unwrap();
        let mut events = events.into_iter();

        let event = events.next().unwrap();
        let log = event.as_log();
        let _n = log.get_field(log_schema().message_key()).unwrap();
        assert_eq!(
            log.get_field(log_schema().message_key()).unwrap(),
            &Value::from("foo")
        );

        assert_eq!(events.next(), None);
    }
}
