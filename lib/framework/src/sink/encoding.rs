use std::io::{self, Write};

use bytes::BytesMut;
use codecs::encoding::{Framer, Transformer};
use event::Event;
use tokio_util::codec::Encoder as _;

/// TrackWriter is a thin wrapper to track written bytes.
pub struct TrackedWriter<W> {
    writer: W,
    written: usize,
}

impl<W> TrackedWriter<W>
where
    W: Write,
{
    pub fn new(writer: W) -> Self {
        Self { writer, written: 0 }
    }

    #[inline]
    pub fn written(&self) -> usize {
        self.written
    }
}

impl<W: Write> Write for TrackedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let count = buf.len();

        self.writer.write_all(buf)?;
        self.written += count;

        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub trait Encoder<T> {
    /// Encodes the input into the provided writer.
    ///
    /// If an I/O error is encountered while encoding the input, an error variant will be returned.
    fn encode(&self, input: T, writer: &mut dyn Write) -> io::Result<usize>;
}

impl Encoder<Vec<Event>> for (Transformer, codecs::Encoder<Framer>) {
    fn encode(&self, mut events: Vec<Event>, writer: &mut dyn Write) -> io::Result<usize> {
        let mut encoder = self.1.clone();
        let mut written = 0;
        let batch_prefix = encoder.batch_prefix();

        writer.write_all(batch_prefix)?;
        written += batch_prefix.len();

        if let Some(last) = events.pop() {
            for mut event in events {
                self.0.transform(&mut event);
                let mut buf = BytesMut::new();

                encoder
                    .encode(event, &mut buf)
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

                writer.write_all(&buf)?;
                written += buf.len();
            }

            let mut event = last;
            self.0.transform(&mut event);
            let mut buf = BytesMut::new();
            encoder
                .serialize(event, &mut buf)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            writer.write_all(&buf)?;
            written += buf.len();
        }

        let batch_suffix = encoder.batch_suffix();
        writer.write_all(batch_suffix)?;
        written += batch_suffix.len();

        Ok(written)
    }
}

impl Encoder<Event> for (Transformer, codecs::encoding::Encoder<()>) {
    fn encode(&self, mut event: Event, writer: &mut dyn Write) -> io::Result<usize> {
        let mut encoder = self.1.clone();

        self.0.transform(&mut event);

        let mut buf = BytesMut::new();
        encoder
            .serialize(event, &mut buf)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        writer.write_all(&buf)?;
        Ok(buf.len())
    }
}

pub fn as_tracked_write<F, I, E>(inner: &mut dyn Write, input: I, f: F) -> io::Result<usize>
where
    F: FnOnce(&mut dyn Write, I) -> Result<(), E>,
    E: Into<io::Error> + 'static,
{
    let mut tracked = TrackedWriter::new(inner);
    f(&mut tracked, input).map_err(|err| err.into())?;
    Ok(tracked.written)
}

/// NoopEncoder is a no op encoder for Encoder implement, it is very useful
/// for batching only RequestBuilder.
pub struct NoopEncoder;

impl<T> Encoder<T> for NoopEncoder {
    fn encode(&self, _input: T, _writer: &mut dyn Write) -> io::Result<usize> {
        panic!("NoopEncoder should never be called")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecs::encoding::{CharacterDelimitedEncoder, JsonSerializer};
    use event::log::Value;
    use std::collections::BTreeMap;

    #[test]
    fn encode_json() {
        for (name, input, want) in [
            ("empty", vec![], (2, "[]")),
            (
                "single",
                vec![Event::Log(
                    BTreeMap::from([(String::from("key"), Value::from("value"))]).into(),
                )],
                (17, r#"[{"key":"value"}]"#),
            ),
            (
                "multiple",
                vec![
                    BTreeMap::from([(String::from("key"), Value::from("value1"))]).into(),
                    BTreeMap::from([(String::from("key"), Value::from("value2"))]).into(),
                    BTreeMap::from([(String::from("key"), Value::from("value3"))]).into(),
                ],
                (
                    52,
                    r#"[{"key":"value1"},{"key":"value2"},{"key":"value3"}]"#,
                ),
            ),
        ] {
            let mut writer = Vec::new();
            let encoding = (
                Transformer::default(),
                codecs::Encoder::<Framer>::new(
                    CharacterDelimitedEncoder::new(b',').into(),
                    JsonSerializer { pretty: false }.into(),
                ),
            );

            let written = encoding.encode(input, &mut writer).unwrap();
            assert_eq!(want.0, written, "test: {name}");
            assert_eq!(String::from_utf8(writer).unwrap(), want.1, "test: {name}");
        }
    }
}
