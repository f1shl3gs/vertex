use bytes::{Buf, Bytes, BytesMut};
use memchr::memchr;
use tokio_util::codec::Decoder;

use super::Error;

#[derive(Clone)]
pub struct NewlineDecoder {
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
}

impl NewlineDecoder {
    pub fn new(max_length: usize) -> NewlineDecoder {
        Self {
            max_length,
            next_index: 0,
            is_discarding: false,
        }
    }
}

impl Decoder for NewlineDecoder {
    type Item = (Bytes, usize);

    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            // Determine how far into the buffer we'll search for a newline. If
            // there's no max_length set, we'll read to the end of the buffer.
            let read_to = std::cmp::min(self.max_length.saturating_add(1), buf.len());

            let newline_offset = memchr(b'\n', &buf[self.next_index..read_to]);
            match (self.is_discarding, newline_offset) {
                (true, Some(offset)) => {
                    // If we found a newline, discard up to that offset and
                    // then stop discarding. On the next iteration, we'll try
                    // to read a line normally.
                    buf.advance(offset + self.next_index + 1);
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
                    buf.advance(1);
                    let size = data.len() + 1;
                    return Ok(Some((data, size)));
                }
                (false, None) if buf.len() > self.max_length => {
                    // Reached the maximum length without finding a
                    // newline, return an error and start discarding on the
                    // next call.
                    self.is_discarding = true;
                    return Err(Error::MaxLengthExceeded);
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
                if buf.is_empty() || buf == &b"\r"[..] {
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use tokio_util::codec::FramedRead;

    #[test]
    fn decode() {
        let mut buf = BytesMut::from("foo\nbar\nabcd");
        let mut decoder = NewlineDecoder::new(1024);
        let (data, size) = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(&data, b"foo".as_ref());
        assert_eq!(size, 4); // `\n` is included

        let (data, size) = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(&data, b"bar".as_ref());
        assert_eq!(size, 4);

        let (data, size) = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(&data, b"abcd".as_ref());
        assert_eq!(size, 4);
    }

    #[tokio::test]
    async fn framed_read() {
        let decoder = NewlineDecoder::new(1024);

        let mut stream = FramedRead::new(b"foo\nbar".as_ref(), decoder);
        let (data, size) = stream.next().await.unwrap().unwrap();
        assert_eq!(&data, b"foo".as_ref());
        assert_eq!(size, 4);

        let (data, size) = stream.next().await.unwrap().unwrap();
        assert_eq!(&data, b"bar".as_ref());
        assert_eq!(size, 3);
    }
}
