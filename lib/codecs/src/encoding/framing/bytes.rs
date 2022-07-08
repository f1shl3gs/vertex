use bytes::BytesMut;
use tokio_util::codec::Encoder;

use crate::error::FramingError;

/// An encoder for handling of plain bytes.
///
/// This encoder does nothing, really. It mainly exists as a symmetric
/// counterpart to `BytesDeserializer`. `BytesEncoder` can be used to
/// explicitly disable framing for formats that encode intrinsic length
/// information - since a sink might set a framing configuration by
/// default depending on the streaming or message based nature of the
/// sink.
#[derive(Debug, Clone)]
pub struct BytesEncoder;

impl BytesEncoder {
    /// Creates a `BytesEncoder`
    pub const fn new() -> Self {
        Self
    }
}

impl Encoder<()> for BytesEncoder {
    type Error = FramingError;

    fn encode(&mut self, _item: (), _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let mut codec = BytesEncoder::new();
        let mut buffer = BytesMut::from("abc");
        codec.encode((), &mut buffer).unwrap();
        assert_eq!(b"abc", &buffer[..]);
    }
}
