use bytes::{BufMut, BytesMut};
use event::Event;
use tokio_util::codec::Encoder;

/// Serializer that converts an `Event` to bytes using the JSON format.
#[derive(Clone, Debug)]
pub struct JsonSerializer;

impl Encoder<Event> for JsonSerializer {
    type Error = crate::Error;

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
    use super::*;
    use event::fields;

    #[test]
    fn serialize() {
        let event = Event::from(fields!(
            "foo" => "bar"
        ));
        let mut serializer = JsonSerializer::new();
        let mut bytes = BytesMut::new();

        serializer.encode(event, &mut bytes).unwrap();
        let encoded = bytes.freeze();
        assert_eq!(encoded, r#"{"tags":{},"fields":{"foo":"bar"}}"#);
    }
}
