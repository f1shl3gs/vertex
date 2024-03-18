use bytes::BytesMut;
use tokio_util::codec::{Encoder, LengthDelimitedCodec};

use crate::FramingError;

/// Config used to build a `LengthDelimitedEncoder`
#[derive(Clone, Debug, Default)]
pub struct LengthDelimitedEncoder(LengthDelimitedCodec);

impl Encoder<()> for LengthDelimitedEncoder {
    type Error = FramingError;

    fn encode(&mut self, _item: (), buffer: &mut BytesMut) -> Result<(), Self::Error> {
        let buf = buffer.split().freeze();
        self.0.encode(buf, buffer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let mut encoder = LengthDelimitedEncoder::default();

        let mut buf = BytesMut::from("abc");
        encoder.encode((), &mut buf).unwrap();

        assert_eq!(&buf[..], b"\0\0\0\x03abc");
    }
}
