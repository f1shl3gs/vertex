use bytes::Bytes;
use event::Event;
use smallvec::{smallvec, SmallVec};

use super::{DeserializeError, Deserializer};

/// Deserializer that converts bytes to an `Event`.
///
/// This deserializer can be considered as the no-op action for input where no
/// further decoding has been specified.
#[derive(Clone, Debug)]
pub struct BytesDeserializer;

impl Deserializer for BytesDeserializer {
    fn parse(&self, buf: Bytes) -> Result<SmallVec<[Event; 1]>, DeserializeError> {
        Ok(smallvec![Event::from(buf)])
    }
}

#[cfg(test)]
mod tests {
    use event::event_path;
    use event::log::Value;

    use super::*;

    #[test]
    fn deserialize() {
        let input = Bytes::from("foo");
        let deserializer = BytesDeserializer;

        let events = deserializer.parse(input).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].as_log().get(event_path!("message")).unwrap(),
            &Value::from("foo")
        );
    }
}
