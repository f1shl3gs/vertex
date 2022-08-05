use crate::FramingError;
use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec::Decoder;

use super::character::CharacterDelimitedDecoder;

#[inline]
pub fn skip_serializing_if_default<E: Default + PartialEq>(e: &E) -> bool {
    e == &E::default()
}

/// Config used to build a `NewlineDelimitedDecoder`
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct NewlineDelimitedDecoderConfig {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub(crate) newline_delimited: NewlineDelimitedDecoderOptions,
}

/// Options for building a `CharacterDelimitedDecoder`
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct NewlineDelimitedDecoderOptions {
    /// The maximum length of the byte buffer
    ///
    /// This length does *not* include the trailing delimiter
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    max_length: Option<usize>,
}

impl NewlineDelimitedDecoderOptions {
    /// Creates a `NewlineDelimitedDecoderOptions` with a maximum frame length limit.
    pub const fn new_with_max_length(max_length: usize) -> Self {
        Self {
            max_length: Some(max_length),
        }
    }
}

impl NewlineDelimitedDecoderConfig {
    /// Creates a new `NewlineDelimitedDecoderConfig`
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a `NewlineDelimitedDecoder` with a maximum frame length limit
    pub const fn new_with_max_length(max_length: usize) -> Self {
        Self {
            newline_delimited: { NewlineDelimitedDecoderOptions::new_with_max_length(max_length) },
        }
    }
}

impl NewlineDelimitedDecoderConfig {
    pub(crate) fn build(&self) -> NewlineDelimitedDecoder {
        if let Some(max_length) = self.newline_delimited.max_length {
            NewlineDelimitedDecoder::new_with_max_length(max_length)
        } else {
            NewlineDelimitedDecoder::new()
        }
    }
}

/// A codec for handling bytes that are delimited by (a) newlines
#[derive(Debug, Clone)]
pub struct NewlineDelimitedDecoder(CharacterDelimitedDecoder);

impl NewlineDelimitedDecoder {
    /// Creates a new `NewlineDelimitedDecoder`
    pub const fn new() -> Self {
        Self(CharacterDelimitedDecoder::new(b'\n'))
    }

    /// Creates a `NewlineDelimitedDecoder` with a maximum frame length limit.
    ///
    /// Any frames longer than `max_length` bytes will be discarded entirely
    pub const fn new_with_max_length(max_length: usize) -> Self {
        Self(CharacterDelimitedDecoder::new_with_max_length(
            b'\n', max_length,
        ))
    }
}

impl Default for NewlineDelimitedDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for NewlineDelimitedDecoder {
    type Item = Bytes;
    type Error = FramingError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.0.decode(src)
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.0.decode_eof(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bytes_with_newlines() {
        let mut input = BytesMut::from("foo\nbar\nbaz");
        let mut decoder = NewlineDelimitedDecoder::new();

        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "bar");
        assert_eq!(decoder.decode(&mut input).unwrap(), None);
    }

    #[test]
    fn decode_bytes_with_newlines_trailing() {
        let mut input = BytesMut::from("foo\nbar\nbaz\n");
        let mut decoder = NewlineDelimitedDecoder::new();

        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "bar");
        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "baz");
        assert_eq!(decoder.decode(&mut input).unwrap(), None);
    }

    #[test]
    fn decode_bytes_with_newlines_and_max_length() {
        let mut input = BytesMut::from("foo\nbarbara\nbaz\n");
        let mut decoder = NewlineDelimitedDecoder::new_with_max_length(3);

        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode(&mut input).unwrap().unwrap(), "baz");
        assert_eq!(decoder.decode(&mut input).unwrap(), None);
    }

    #[test]
    fn decode_eof_bytes_with_newlines() {
        let mut input = BytesMut::from("foo\nbar\nbaz");
        let mut decoder = NewlineDelimitedDecoder::new();

        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "bar");
        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "baz");
    }

    #[test]
    fn decode_eof_bytes_with_newlines_trailing() {
        let mut input = BytesMut::from("foo\nbar\nbaz\n");
        let mut decoder = NewlineDelimitedDecoder::new();

        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "bar");
        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "baz");
        assert_eq!(decoder.decode_eof(&mut input).unwrap(), None);
    }

    #[test]
    fn decode_eof_bytes_with_newlines_and_max_length() {
        let mut input = BytesMut::from("foo\nbarbara\nbaz\n");
        let mut decoder = NewlineDelimitedDecoder::new_with_max_length(3);

        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "foo");
        assert_eq!(decoder.decode_eof(&mut input).unwrap().unwrap(), "baz");
        assert_eq!(decoder.decode_eof(&mut input).unwrap(), None);
    }
}
