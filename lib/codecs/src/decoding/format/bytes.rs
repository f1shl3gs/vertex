use bytes::Bytes;
use event::{Event, LogRecord};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use crate::decoding::format::{DeserializeError, Deserializer};

/// Deserializer that converts bytes to an `Event`.
///
/// This deserializer can be considered as the no-op action for input where no
/// further decoding has been specified.
#[derive(Clone, Debug)]
pub struct BytesDeserializer;

impl BytesDeserializer {
    /// Creates a new `BytesDeserializer`
    pub fn new() -> Self {
        Self
    }
}

impl Deserializer for BytesDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        Ok(smallvec![Event::from(buf)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::log::Value;

    #[test]
    fn deserialize() {
        let input = Bytes::from("foo");
        let deserializer = BytesDeserializer;

        let events = deserializer.parse(input).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].as_log().get_field("message").unwrap(),
            &Value::from("foo")
        );
    }
}
