use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::Encoder;

use crate::serde::ascii_char;
use crate::FramingError;

/// Config used to build a `CharacterDelimitedEncoder`
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CharacterDelimitedFramerConfig {
    /// Options for the character delimited encoder.
    #[serde(with = "ascii_char")]
    pub delimiter: u8,
}

impl CharacterDelimitedFramerConfig {
    /// Creates a `CharacterDelimitedFramerConfig` with the specified delimiter.
    pub const fn new(delimiter: u8) -> Self {
        Self { delimiter }
    }

    /// Build the `CharacterDelimitedEncoder` from this configuration.
    pub const fn build(&self) -> CharacterDelimitedEncoder {
        CharacterDelimitedEncoder {
            delimiter: self.delimiter,
        }
    }
}

/// An encoder for handling bytes that are delimited by (a) chosen character(s).
#[derive(Clone, Debug)]
pub struct CharacterDelimitedEncoder {
    /// The character that delimits byte sequences.
    pub delimiter: u8,
}

impl CharacterDelimitedEncoder {
    /// Creates a new `CharacterDelimitedEncoder` with the delimiter.
    pub const fn new(delimiter: u8) -> Self {
        Self { delimiter }
    }
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
