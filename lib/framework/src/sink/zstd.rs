use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Write;

use super::buffer::CompressionLevel;

pub struct ZstdCompressionLevel(i32);

impl From<CompressionLevel> for ZstdCompressionLevel {
    fn from(value: CompressionLevel) -> Self {
        let val: i32 = match value {
            CompressionLevel::None => 0,
            CompressionLevel::Default => zstd::DEFAULT_COMPRESSION_LEVEL,
            CompressionLevel::Best => 21,
            CompressionLevel::Fast => 1,
            CompressionLevel::Value(v) => v.clamp(1, 21) as i32,
        };

        ZstdCompressionLevel(val)
    }
}

impl Display for ZstdCompressionLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct ZstdEncoder<W: Write> {
    inner: zstd::Encoder<'static, W>,
}

impl<W: Write> ZstdEncoder<W> {
    pub fn new(writer: W, level: ZstdCompressionLevel) -> io::Result<Self> {
        let encoder = zstd::Encoder::new(writer, level.0)?;
        Ok(Self { inner: encoder })
    }

    pub fn finish(self) -> io::Result<W> {
        self.inner.finish()
    }

    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }
}

impl<W: Write> Write for ZstdEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Caller handles the result of `write`
        #[allow(clippy::disallowed_methods)]
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: Debug + Write> Debug for ZstdEncoder<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ZstdEncoder")
            .field("inner", &self.get_ref())
            .finish()
    }
}

/// Safety:
/// 1. There is no sharing references to zstd encoder. `Write` requires
///    unique reference, and `finish` moves the instance itself.
/// 2. Sharing only internal writer, which implements `Sync`
unsafe impl<W: Write + Sync> Sync for ZstdEncoder<W> {}
