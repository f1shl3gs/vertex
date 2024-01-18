use bytes::{BufMut, BytesMut};
use flate2::write::{GzEncoder, ZlibEncoder};
use std::io::Write;

use super::buffer::Compression;

const BUFFER_SIZE: usize = 1024;

enum Writer {
    Plain(bytes::buf::Writer<BytesMut>),
    Gzip(GzEncoder<bytes::buf::Writer<BytesMut>>),
    Zlib(ZlibEncoder<bytes::buf::Writer<BytesMut>>),
}

impl Writer {
    pub fn get_ref(&self) -> &BytesMut {
        match self {
            Writer::Plain(inner) => inner.get_ref(),
            Writer::Gzip(inner) => inner.get_ref().get_ref(),
            Writer::Zlib(inner) => inner.get_ref().get_ref(),
        }
    }
}

impl From<Compression> for Writer {
    fn from(compression: Compression) -> Self {
        let writer = BytesMut::with_capacity(BUFFER_SIZE).writer();

        match compression {
            Compression::None => Writer::Plain(writer),
            Compression::Gzip(level) => Writer::Gzip(GzEncoder::new(writer, level)),
            Compression::Zlib(level) => Writer::Zlib(ZlibEncoder::new(writer, level)),
        }
    }
}

impl Write for Writer {
    #[allow(clippy::disallowed_methods)] // Caller handles the result of `write`.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Writer::Plain(inner) => inner.write(buf),
            Writer::Gzip(writer) => writer.write(buf),
            Writer::Zlib(writer) => writer.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Writer::Plain(writer) => writer.flush(),
            Writer::Gzip(writer) => writer.flush(),
            Writer::Zlib(writer) => writer.flush(),
        }
    }
}

/// Simple compressor implementation based on `Compression`.
///
/// Users can acquire a `Compressor` via `Compressor::from` based on the desired compression
/// scheme.
pub struct Compressor {
    compression: Compression,
    inner: Writer,
}

impl Compressor {
    /// Gets a mutable reference to the underlying buffer.
    pub fn get_ref(&self) -> &BytesMut {
        self.inner.get_ref()
    }

    /// Gets whether or not this compressor will actually compress the input.
    ///
    /// While it may be counterintuitive for "compression" ot not compress, this is simply a
    /// consequence of designing a single type that may or may not compress so that we can
    /// avoid having to box writers at a higher-level.
    ///
    /// Some callers can benefit from knowing whether or not compression is actually taking
    /// place, as different size limitations may come into paly.
    pub const fn is_compressed(&self) -> bool {
        self.compression.is_compressed()
    }

    /// Consumes the compressor, returning the internal buffer used by the compressor.
    ///
    /// # Errors
    ///
    /// If the compressor encounters an I/O error while finalizing the payload, an error
    /// variant will be returned.
    pub fn finish(self) -> std::io::Result<BytesMut> {
        let buf = match self.inner {
            Writer::Plain(writer) => writer,
            Writer::Gzip(writer) => writer.finish()?,
            Writer::Zlib(writer) => writer.finish()?,
        };

        Ok(buf.into_inner())
    }

    /// Consumes the compressor, returning the internal buffer used by the compressor.
    ///
    /// # Panics
    ///
    /// Panics if finalizing the compressor encounters an I/O error. This should generally
    /// only be possible when the system is out of memory and allocation cannot be performed
    /// to write any footer/checksum data.
    ///
    /// Consider using `finish` if catching these scenarios is important
    pub fn into_inner(self) -> BytesMut {
        match self.inner {
            Writer::Plain(writer) => writer,
            Writer::Gzip(writer) => writer
                .finish()
                .expect("gzip writer should not fail to finish"),
            Writer::Zlib(writer) => writer
                .finish()
                .expect("zlib writer should not fail to finish"),
        }
        .into_inner()
    }
}

impl Write for Compressor {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        #[allow(clippy::disallowed_methods)] // Caller handles the result of `write`.
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl From<Compression> for Compressor {
    fn from(compression: Compression) -> Self {
        Compressor {
            compression,
            inner: compression.into(),
        }
    }
}