use std::fmt::{Display, Formatter};

use bytes::{Buf, Bytes, BytesMut};
use memchr::memmem::Finder;
use tokio_util::codec::Decoder;

pub struct BytesDelimitDecoder {
    // Stored index of the next index to examine for a `\n` character.
    // This is used to optimize searching.
    // For example, if `decode` was called with `abc`, it would hold `3`,
    // because that is the next index to examine.
    // The next time `decode` is called with `abcde\n`, the method will
    // only look at `de\n` before returning.
    next_index: usize,

    /// The maximum length for a given line. If `usize::MAX`, lines will be
    /// read until a `\n` character is reached.
    max_length: usize,

    /// Are we currently discarding the remainder of a line which was over
    /// the length limit?
    is_discarding: bool,

    delimiter: Finder<'static>,
}

impl BytesDelimitDecoder {
    #[inline]
    pub fn new(delimiter: Vec<u8>, max_length: usize) -> Self {
        let delimiter = Finder::new(&delimiter).into_owned();

        Self {
            delimiter,
            max_length,
            next_index: 0,
            is_discarding: false,
        }
    }
}

/// An error occurred while encoding or decoding a line.
#[derive(Debug)]
pub enum LinesCodecError {
    /// The maximum line length was exceeded.
    MaxLineLengthExceeded,

    /// An IO error occurred.
    Io(std::io::Error),
}

impl From<std::io::Error> for LinesCodecError {
    fn from(err: std::io::Error) -> Self {
        LinesCodecError::Io(err)
    }
}

impl Display for LinesCodecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LinesCodecError::MaxLineLengthExceeded => f.write_str("Maximum line length exceeded"),
            LinesCodecError::Io(err) => err.fmt(f),
        }
    }
}

impl Decoder for BytesDelimitDecoder {
    type Item = (Bytes, usize);

    type Error = LinesCodecError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let delim_len = self.delimiter.needle().len();

        loop {
            // Determine how far into the buffer we'll search for a newline. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = std::cmp::min(self.max_length.saturating_add(delim_len), buf.len());

            let newline_offset = self.delimiter.find(&buf[self.next_index..read_to]);
            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a line normally.
                    buf.advance(offset + self.next_index + delim_len);
                    self.is_discarding = false;
                    self.next_index = 0;
                }
                (true, None) => {
                    // Otherwise, we didn't find a newline, so we'll discard
                    // everything we read. On the next iteration, we'll continue
                    // discarding up to max_len bytes unless we find a newline.
                    buf.advance(read_to);
                    self.next_index = 0;
                    if buf.is_empty() {
                        return Ok(None);
                    }
                }
                (false, Some(offset)) => {
                    // Found a line!
                    let newline_index = offset + self.next_index;
                    self.next_index = 0;
                    let data = buf.split_to(newline_index).freeze();
                    buf.advance(delim_len);
                    let size = data.len() + delim_len;
                    return Ok(Some((data, size)));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(LinesCodecError::MaxLineLengthExceeded);
                }
                (false, None) => {
                    // We didn't find a line or reach the length limit, so the next
                    // call will resume searching at the current offset.
                    self.next_index = read_to;
                    return Ok(None);
                }
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.decode(buf)? {
            Some(frame) => Ok(Some(frame)),
            None => {
                self.next_index = 0;
                // No terminating newline - return remaining data, if any
                if buf.is_empty() || buf == self.delimiter.needle() {
                    Ok(None)
                } else {
                    let data = buf.split_to(buf.len()).freeze();
                    let size = data.len();
                    Ok(Some((data, size)))
                }
            }
        }
    }
}
