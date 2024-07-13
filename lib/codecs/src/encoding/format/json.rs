use bytes::{BufMut, BytesMut};
use configurable::Configurable;
use event::Event;
use serde::{Deserialize, Serialize};
use tokio_util::codec::Encoder;

use super::SerializeError;

/// Config used to build a `JsonSerializer`
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct JsonSerializerConfig {
    /// Whether to use pretty JSON formatting.
    #[serde(default)]
    pub pretty: bool,
}

/// Serializer that converts an `Event` to bytes using the JSON format.
#[derive(Clone, Debug)]
pub struct JsonSerializer {
    /// Whether to use pretty JSON formatting.
    pub pretty: bool,
}

impl JsonSerializer {
    /// Creates a new `JsonSerializer`
    pub const fn new(pretty: bool) -> Self {
        JsonSerializer { pretty }
    }
}

impl Encoder<Event> for JsonSerializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let writer = dst.writer();

        if self.pretty {
            match event {
                Event::Log(log) => serde_json::to_writer_pretty(writer, &log),
                Event::Metric(metric) => serde_json::to_writer_pretty(writer, &metric),
                Event::Trace(trace) => serde_json::to_writer_pretty(writer, &trace),
            }
        } else {
            match event {
                Event::Log(log) => serde_json::to_writer(writer, &log),
                Event::Metric(metric) => serde_json::to_writer(writer, &metric),
                Event::Trace(trace) => serde_json::to_writer(writer, &trace),
            }
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
        let mut serializer = JsonSerializer::new(false);
        let mut bytes = BytesMut::new();

        serializer.encode(event, &mut bytes).unwrap();
        let encoded = bytes.freeze();
        assert_eq!(encoded, r#"{"foo":"bar"}"#);
    }
}
