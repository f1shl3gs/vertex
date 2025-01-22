use bytes::Bytes;
use event::Events;

use super::{DeserializeError, Deserializer};

/// Deserializer that converts bytes to an `Event`.
///
/// This deserializer can be considered as the no-op action for input where no
/// further decoding has been specified.
#[derive(Clone, Debug)]
pub struct BytesDeserializer;

impl Deserializer for BytesDeserializer {
    fn parse(&self, buf: Bytes) -> Result<Events, DeserializeError> {
        Ok(Events::Logs(vec![buf.into()]))
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

        let logs = deserializer.parse(input).unwrap().into_logs().unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(
            logs[0].get(event_path!("message")).unwrap(),
            &Value::from("foo")
        );
    }
}
