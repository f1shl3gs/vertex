use bytes::{BufMut, BytesMut};
use event::Event;
use log_schema::log_schema;
use tokio_util::codec::Encoder;

use super::SerializeError;

/// Serializer that converts a log to bytes by extracting the message key, or converts a metric to
/// bytes by calling its `Display` implementation.
#[derive(Clone, Debug)]
pub struct TextSerializer;

impl TextSerializer {
    /// Creates a new `TextSerializer`
    pub const fn new() -> Self {
        Self
    }
}

impl Encoder<Event> for TextSerializer {
    type Error = SerializeError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match event {
            Event::Log(log) => {
                let message_key = log_schema().message_key();

                if let Some(bytes) = log
                    .get_field(message_key)
                    .map(|value| value.coerce_to_bytes())
                {
                    dst.put(bytes);
                }
            }
            Event::Metric(metric) => {
                let bytes = metric.to_string();
                dst.put(bytes.as_ref());
            }
            Event::Trace(_trace) => {
                // TODO
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use event::{fields, tags, Metric};

    use super::*;

    #[test]
    fn serialize() {
        let tests = [
            (
                "log",
                Event::from(fields!(
                    "foo" => "bar",
                    log_schema().message_key() => "msg"
                )),
                "msg",
            ),
            (
                "metric",
                Event::from(Metric::gauge_with_tags(
                    "metric",
                    "desc",
                    1.0,
                    tags!(
                        "foo" => "bar"
                    ),
                )),
                "metric{foo=\"bar\"} 1",
            ),
        ];

        for (name, event, want) in tests {
            let mut serializer = TextSerializer::new();
            let mut buf = BytesMut::new();

            serializer.encode(event, &mut buf).unwrap();

            assert_eq!(buf.freeze(), want, "case {}", name);
        }
    }
}
