use bytes::{BufMut, BytesMut};
use event::Event;
use tokio_util::codec::Encoder;

use super::SerializeError;

/// Serializer that converts an `Event` to bytes using the JSON format.
#[derive(Clone, Debug)]
pub struct JsonSerializer;

impl JsonSerializer {
    /// Creates a new `JsonSerializer`
    pub const fn new() -> Self {
        JsonSerializer
    }
}

impl Encoder<Event> for JsonSerializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let writer = dst.writer();

        match event {
            Event::Log(log) => serde_json::to_writer(writer, &log),
            Event::Metric(metric) => serde_json::to_writer(writer, &metric),
            Event::Trace(trace) => serde_json::to_writer(writer, &trace),
        }
        .map_err(Into::into)
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
        let mut serializer = JsonSerializer;
        let mut bytes = BytesMut::new();

        serializer.encode(event, &mut bytes).unwrap();
        let encoded = bytes.freeze();
        assert_eq!(encoded, r#"{"foo":"bar"}"#);
    }
}
