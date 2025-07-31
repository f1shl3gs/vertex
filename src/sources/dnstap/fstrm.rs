//! A frame streams data transport protocol.
//!
//! https://github.com/farsightsec/fstrm

use bytes::{Buf, Bytes, BytesMut};

pub enum DecoderError {
    LimitExceed(usize),

    Stopped,

    Io(std::io::Error),
}

impl From<std::io::Error> for DecoderError {
    fn from(err: std::io::Error) -> Self {
        DecoderError::Io(err)
    }
}

pub struct FStrmDecoder {
    limit: usize,
    expect_stop: bool,
}

impl FStrmDecoder {
    pub fn new(limit: usize) -> Self {
        FStrmDecoder {
            limit,
            expect_stop: false,
        }
    }
}

impl tokio_util::codec::Decoder for FStrmDecoder {
    type Item = (Bytes, usize);
    type Error = DecoderError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        let len = u32::from_be_bytes(src[..4].try_into().unwrap()) as usize;
        if len == 0 {
            self.expect_stop = true;
            return Ok(None);
        }

        if self.limit < len {
            return Err(DecoderError::LimitExceed(len));
        }

        if src.len() <= len + 4 {
            return Ok(None);
        }

        src.advance(4);
        let mut frame = src.split_to(len).freeze();

        if self.expect_stop {
            return match frame.try_get_u32() {
                Ok(typ) => {
                    if typ == crate::sources::dnstap::CONTROL_STOP {
                        return Err(DecoderError::Stopped);
                    }

                    Err(DecoderError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("want STOP control frame, but got {typ}"),
                    )))
                }
                Err(err) => Err(DecoderError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err,
                ))),
            };
        }

        Ok(Some((frame, len)))
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        self.decode(src)
    }
}
