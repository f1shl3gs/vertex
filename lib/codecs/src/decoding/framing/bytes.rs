use bytes::{Bytes, BytesMut};
use tokio_util::codec::Decoder;

use super::FramingError;

/// A decoder for passing through bytes as-is.
///
/// This is basically a no-op and is used to convert from `BytesMut` to `Bytes`
#[derive(Clone, Debug)]
pub struct BytesDeserializerConfig {
    /// Whether the empty buffer has been flushed. This is important to
    /// propagate empty frames in message based transports.
    flushed: bool,
}

impl BytesDeserializerConfig {
    /// Create a new `BytesDecoder`.
    pub const fn new() -> Self {
        Self { flushed: false }
    }
}

impl Decoder for BytesDeserializerConfig {
    type Item = Bytes;
    type Error = FramingError;

    fn decode(&mut self, _src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.flushed = false;
        Ok(None)
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.flushed && buf.is_empty() {
            Ok(None)
        } else {
            self.flushed = true;
            let frame = buf.split();
            Ok(Some(frame.freeze()))
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use tokio_util::codec::FramedRead;

    use super::*;

    #[test]
    fn decode() {
        let mut input = BytesMut::from("some bytes");
        let mut decoder = BytesDeserializerConfig::new();

        assert_eq!(decoder.decode(&mut input).unwrap(), None);
        assert_eq!(
            decoder.decode_eof(&mut input).unwrap().unwrap(),
            "some bytes"
        );
        assert_eq!(decoder.decode(&mut input).unwrap(), None);
    }

    #[tokio::test]
    async fn decode_frame_reader() {
        let input: &[u8] = b"foo";
        let decoder = BytesDeserializerConfig::new();

        let mut reader = FramedRead::new(input, decoder);

        assert_eq!(reader.next().await.unwrap().unwrap(), "foo");
        assert!(reader.next().await.is_none());
    }

    #[tokio::test]
    async fn decode_empty() {
        let input: &[u8] = b"";
        let decoder = BytesDeserializerConfig::new();

        let mut reader = FramedRead::new(input, decoder);

        assert_eq!(reader.next().await.unwrap().unwrap(), "");
        assert!(reader.next().await.is_none());
    }
}
