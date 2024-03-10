use std::fmt::{Debug, Formatter};
use std::io;
use std::io::Write;

use snap::raw::Encoder;

pub struct SnappyEncoder<W: Write> {
    writer: W,
    buffer: Vec<u8>,
}

impl<W: Write> SnappyEncoder<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            buffer: vec![],
        }
    }

    pub fn finish(mut self) -> io::Result<W> {
        let mut encoder = Encoder::new();
        let compressed = encoder.compress_vec(&self.buffer)?;

        self.writer.write_all(&compressed)?;

        Ok(self.writer)
    }

    pub const fn get_ref(&self) -> &W {
        &self.writer
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl<W: Write> Write for SnappyEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W: Write + Debug> Debug for SnappyEncoder<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SnappyEncoder")
            .field("inner", &self.get_ref())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    #[test]
    fn is_empty() {
        let writer = BytesMut::with_capacity(64).writer();
        let mut encoder = SnappyEncoder::new(writer);

        encoder.write_all(b"blah blah blah").unwrap();

        // Because we are buffering the results until the end, the writer will be
        // empty, but our buffer won't be. The 'is_empty' function is provided to
        // allow us to determine if data has been written to the encoder without
        // having to check the writer.
        assert!(encoder.get_ref().get_ref().is_empty());
        assert!(encoder.is_empty());
    }
}
