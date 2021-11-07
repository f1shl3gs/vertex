use std::io::{self, Write};

use serde::{Deserialize, Serialize};
use log_schema::log_schema;

use crate::encoding::Encoder;
use crate::{Event, LogRecord};


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
    fn single_trailer_hook(self, writer: &mut dyn io::Write) -> io::Result<usize> {
        let buf = match self {
            StandardEncodings::NdJson => Some(&[b'\n']),
            _ => None
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
            StandardEncodings::Json | StandardEncodings::NdJson => DEFAULT_JSON_ENCODER.encode(input, writer),
        };

        written += n;
        let n = self.single_trailer_hook(writer)?;
        written += n;

        Ok(written)
    }
}

impl Encoder<Vec<Event>> for StandardEncodings {
    fn encode(&self, input: Vec<Event>, writer: &mut dyn Write) -> io::Result<usize> {
        todo!()
    }
}

fn as_tracked_write<F, I, E>(inner: &mut dyn io::Write, input: I, f: F) -> io::Result<usize>
    where
        F: FnOnce(&mut dyn io::Write, I) -> Result<(), E>,
        E: Into<io::Error> + 'static
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
    let _ = f(&mut tracked, input).map_err(|err| err.into())?;
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
        as_tr
    }
}

impl Encoder<Event> for StandardJsonEncoding {
    fn encode(&self, input: Event, writer: &mut dyn Write) -> std::io::Result<usize> {
        match input {
            Event::Log(log) => self.encode(log, writer),
            Event::Metric(metric) => as_tracked_write(writer, &metric, |writer, item| {
                serde_json::to_writer(writer, item)
            })
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
    fn encode(&self, input: Event, writer: &mut dyn Write) -> io::Result<usize> {
        match event {
            Event::Log(log) => {
                let msg = log.get_field(log_schema().message_key())
                    .map(|v| v.as_bytes())
                    .unwrap_or_default();
                writer.write_all(&msg[..])
                    .map(|()| msg.len())
            }

            Event::Metric(metric) => {
                let msg = metric.to_string().into_bytes();
                writer.write_all(&msg).map(|()| msg.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}