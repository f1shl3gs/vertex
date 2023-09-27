use bytes::{BufMut, BytesMut};
use event::Event;
use tokio_util::codec::Encoder;

use super::SerializeError;

/// Serializer that converts an `Event` to bytes using the JSON format
#[derive(Clone, Debug)]
pub struct NativeJsonSerializer;

impl NativeJsonSerializer {
    /// Creates a new `NativeJsonSerializer`
    pub const fn new() -> Self {
        Self
    }
}

impl Encoder<Event> for NativeJsonSerializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let writer = dst.writer();
        serde_json::to_writer(writer, &event).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use event::fields;

    use super::*;

    #[test]
    fn serialize() {
        let event = Event::from(fields!(
            "foo" => "bar"
        ));

        let mut serializer = NativeJsonSerializer::new();
        let mut buf = BytesMut::new();

        serializer.encode(event, &mut buf).unwrap();
        assert_eq!(buf.freeze(), r#"{"log":{"foo":"bar"}}"#)
    }
}
