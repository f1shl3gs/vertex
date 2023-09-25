use bytes::BytesMut;
use tokio_util::codec::Encoder;

use super::character::CharacterDelimitedEncoder;
use crate::FramingError;

/// A codec for handling bytes that are delimited by (a) newline(s).
#[derive(Debug, Clone)]
pub struct NewlineDelimitedEncoder(CharacterDelimitedEncoder);

impl NewlineDelimitedEncoder {
    /// Create a new `NewlineDelimitedEncoder`
    pub const fn new() -> Self {
        Self(CharacterDelimitedEncoder::new(b'\n'))
    }
}

impl Encoder<()> for NewlineDelimitedEncoder {
    type Error = FramingError;

    fn encode(&mut self, item: (), dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.0.encode(item, dst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let mut buf = BytesMut::from("foo");
        let mut encoder = NewlineDelimitedEncoder::new();

        encoder.encode((), &mut buf).unwrap();

        assert_eq!(buf, "foo\n");
    }
}
