use std::io::{self, Write};

use event::{Event, LogRecord};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};

use super::Encoder;

static DEFAULT_TEXT_ENCODER: StandardTextEncoding = StandardTextEncoding;
static DEFAULT_JSON_ENCODER: StandardJsonEncoding = StandardJsonEncoding;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StandardEncodings {
    Text,
    Json,
    NdJson,
}

impl StandardEncodings {
    fn batch_pre_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::Json => Some(&[b'[']),
            _ => None,
        };

        if let Some(buf) = buf {
            writer.write_all(buf).map(|()| buf.len())
        } else {
            Ok(0)
        }
    }

    fn batch_post_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::Json => Some(&[b']']),
            _ => None,
        };

        if let Some(buf) = buf {
            writer.write_all(buf).map(|()| buf.len())
        } else {
            Ok(0)
        }
    }

    fn batch_delimiter_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::Json => Some(&[b',']),
            StandardEncodings::Text => Some(&[b'\n']),
            _ => None,
        };

        if let Some(buf) = buf {
            writer.write_all(buf).map(|()| buf.len())
        } else {
            Ok(0)
        }
    }

    fn batch_trailer_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::NdJson => Some(&[b'\n']),
            _ => None,
        };

        if let Some(buf) = buf {
            writer.write_all(buf).map(|()| buf.len())
        } else {
            Ok(0)
        }
    }

    fn single_trailer_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::NdJson => Some(&[b'\n']),
            _ => None,
        };

        if let Some(buf) = buf {
            writer.write_all(buf).map(|()| buf.len())
        } else {
            Ok(0)
        }
    }
}

impl Encoder<Event> for StandardEncodings {
    fn encode(&self, input: Event, writer: &mut dyn Write) -> io::Result<usize> {
        let mut written = 0;

        let n = match self {
            StandardEncodings::Text => DEFAULT_TEXT_ENCODER.encode(input, writer),
            StandardEncodings::Json | StandardEncodings::NdJson => {
                DEFAULT_JSON_ENCODER.encode(input, writer)
            }
        }?;

        written += n;
        let n = self.single_trailer_hook(writer)?;
        written += n;

        Ok(written)
    }
}

impl Encoder<Vec<Event>> for StandardEncodings {
    fn encode(&self, input: Vec<Event>, writer: &mut dyn Write) -> io::Result<usize> {
        let mut written = 0;

        let n = self.batch_pre_hook(writer)?;
        written += n;

        let last = input.len();
        for (i, event) in input.into_iter().enumerate() {
            let n = match self {
                StandardEncodings::Text => DEFAULT_TEXT_ENCODER.encode(event, writer),
                StandardEncodings::Json | StandardEncodings::NdJson => {
                    DEFAULT_JSON_ENCODER.encode(event, writer)
                }
            }?;

            written += n;

            if i != last - 1 {
                let n = self.batch_delimiter_hook(writer)?;
                written += n;
            }

            let n = self.batch_trailer_hook(writer)?;
            written += n;
        }

        let n = self.batch_post_hook(writer)?;
        written += n;

        Ok(written)
    }
}

pub fn as_tracked_write<F, I, E>(inner: &mut dyn io::Write, input: I, f: F) -> io::Result<usize>
where
    F: FnOnce(&mut dyn io::Write, I) -> Result<(), E>,
    E: Into<io::Error> + 'static,
{
    struct Tracked<'inner> {
        count: usize,
        inner: &'inner mut dyn io::Write,
    }

    impl<'inner> io::Write for Tracked<'inner> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = self.inner.write(buf)?;
            self.count += n;
            Ok(n)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.flush()
        }
    }

    let mut tracked = Tracked { count: 0, inner };
    f(&mut tracked, input).map_err(|err| err.into())?;
    Ok(tracked.count)
}

/// Standard implementation for encoding events as JSON
///
/// All event types will be serialized to JSON, without pretty printing. Uses
/// [`serde_json::to_writer`] under the hood, so all caveats mentioned therein apply here.
#[derive(PartialEq, Debug, Default)]
pub struct StandardJsonEncoding;

impl Encoder<LogRecord> for StandardJsonEncoding {
    fn encode(&self, input: LogRecord, writer: &mut dyn Write) -> std::io::Result<usize> {
        as_tracked_write(writer, &input, |writer, item| {
            serde_json::to_writer(writer, item)
        })
    }
}

impl Encoder<Event> for StandardJsonEncoding {
    fn encode(&self, input: Event, writer: &mut dyn Write) -> std::io::Result<usize> {
        match input {
            Event::Log(log) => self.encode(log, writer),
            Event::Metric(metric) => as_tracked_write(writer, &metric, |writer, item| {
                serde_json::to_writer(writer, item)
            }),
            Event::Trace(span) => as_tracked_write(writer, &span, |writer, span| {
                serde_json::to_writer(writer, span)
            }),
        }
    }
}

/// Standard implementation for encoding events as text.
///
/// If given a log event, the value used in the field matching the global lob schema's "message"
/// key will be written out, otherwise an empty string will be written. If anything other than
/// a log event is given, the encoder will panic
///
/// Each event is delimited with a newline character
pub struct StandardTextEncoding;

impl Encoder<Event> for StandardTextEncoding {
    fn encode(&self, event: Event, writer: &mut dyn Write) -> io::Result<usize> {
        match event {
            Event::Log(log) => {
                let msg = log
                    .get_field(log_schema().message_key())
                    .map(|v| v.as_bytes())
                    .unwrap_or_default();
                writer.write_all(&msg[..]).map(|()| msg.len())
            }

            Event::Metric(metric) => {
                let msg = metric.to_string().into_bytes();
                writer.write_all(&msg).map(|()| msg.len())
            }

            Event::Trace(span) => as_tracked_write(writer, &span, |writer, span| {
                serde_json::to_writer(writer, span)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::Metric;

    fn encode_event(event: Event, encoding: StandardEncodings) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let result = encoding.encode(event, &mut buf);
        result.map(|_| buf)
    }

    fn encode_events(events: Vec<Event>, encoding: StandardEncodings) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let result = encoding.encode(events, &mut buf);
        result.map(|_| buf)
    }

    #[test]
    fn test_standard_text_log_single() {
        let encoding = StandardEncodings::Text;

        let message = "log event";
        let event = Event::from(message.to_string());
        let result = encode_event(event, encoding).expect("should not have failed");
        let encoded = std::str::from_utf8(&result).expect("result should be valid");

        let expected = message;
        assert_eq!(expected, encoded);
    }

    #[test]
    fn test_standard_text_log_multiple() {
        let encoding = StandardEncodings::Text;

        let message1 = "log event 1";
        let event1 = Event::from(message1);

        let message2 = "log event 2";
        let event2 = Event::from(message2);

        let result = encode_events(vec![event1, event2], encoding).expect("should not have failed");
        let encoded = std::str::from_utf8(&result).expect("result should be valid");

        let expected = format!("{}\n{}", message1, message2);
        assert_eq!(expected, encoded);
    }

    #[test]
    fn test_standard_text_metric_single() {
        let encoding = StandardEncodings::Text;

        let event = Metric::gauge("name", "desc", 1.23).into();

        let result = encode_event(event, encoding).expect("should not have failed");
        let encoded = std::str::from_utf8(&result).expect("result should be valid");

        let expected = "name 1.23";
        assert_eq!(expected, encoded)
    }
}
