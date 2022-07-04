use crate::encoding::framing::FramingError;
use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::Encoder;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CharacterDelimitedFramerConfig {
    pub delimiter: u8,
}

impl CharacterDelimitedFramerConfig {
    pub const fn new(delimiter: u8) -> Self {
        Self { delimiter }
    }

    pub const fn build(&self) -> CharacterDelimitedEncoder {
        CharacterDelimitedEncoder {
            delimiter: self.delimiter,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CharacterDelimitedEncoder {
    delimiter: u8,
}

impl Encoder<()> for CharacterDelimitedEncoder {
    type Error = FramingError;

    fn encode(&mut self, _item: (), dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(self.delimiter);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let mut codec = CharacterDelimitedEncoder { delimiter: b'\n' };
        let mut buf = BytesMut::from("abc");
        codec.encode((), &mut buf).unwrap();

        assert_eq!(b"abc\n", &buf[..]);
    }
}
