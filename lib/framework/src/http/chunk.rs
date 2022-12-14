use bytes::{Buf, Bytes, BytesMut};
use memchr::memchr;
use tokio_util::codec::Decoder;

#[derive(Default, Debug, PartialEq)]
enum ChunkedState {
    #[default]
    SizeCr,
    SizeLf,
    BodyCr,
    BodyLf,
}

#[derive(Default)]
pub struct ChunkedDecoder {
    size: usize,
    state: ChunkedState,
}

impl Decoder for ChunkedDecoder {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self.state {
                ChunkedState::SizeCr => match memchr(b'\r', buf) {
                    None => return Ok(None),
                    Some(next) => {
                        let part = buf.split_to(next).freeze();

                        let size = match parse_hex(part.chunk()) {
                            Ok(n) => n,
                            Err(err) => {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    err,
                                ))
                            }
                        };

                        self.size = size as usize;
                        self.state = ChunkedState::SizeLf;
                        buf.advance(1);
                        continue;
                    }
                },

                ChunkedState::SizeLf => {
                    let char = buf.chunk()[0];

                    if char == b'\n' {
                        buf.advance(1);
                        self.state = ChunkedState::BodyCr;

                        continue;
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Unexpected token {}", char),
                        ));
                    }
                }

                ChunkedState::BodyCr => {
                    if buf.len() < self.size {
                        return Ok(None);
                    }

                    let data = buf.split_to(self.size).freeze();
                    buf.advance(1);
                    self.state = ChunkedState::BodyLf;
                    self.size = 0;

                    return Ok(Some(data));
                }

                ChunkedState::BodyLf => {
                    let char = buf.chunk()[0];

                    if char == b'\n' {
                        buf.advance(1);
                        self.state = ChunkedState::SizeCr;

                        continue;
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Unexpected token {}", char),
                        ));
                    }
                }
            }
        }
    }

    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if self.state == ChunkedState::SizeCr && buf.is_empty() {
            return Ok(None);
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            format!("unexpected eof, want {:?}", self.state),
        ))
    }
}

fn parse_hex(v: &[u8]) -> Result<u64, &str> {
    if v.len() >= 16 {
        return Err("http chunk length too large");
    }

    let mut n: u64 = 0;
    for b in v {
        let mut b = *b;
        b = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return Err("invalid byte in chunk length"),
        };

        n <<= 4;
        n |= b as u64
    }

    Ok(n)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::print_stdout)]

    use super::*;
    use futures_util::TryStreamExt;
    use tokio_util::codec::FramedRead;

    #[test]
    fn hex() {
        let tests = [("0", 0), ("a", 10), ("F", 15), ("10", 16), ("233", 563)];

        for (input, want) in tests {
            let got = parse_hex(input.as_bytes()).unwrap();
            assert_eq!(want, got, "{}", input)
        }
    }

    #[tokio::test]
    async fn good() {
        let input = "7\r\nMozilla\r\n11\r\nDeveloper Network\r\n0\r\n\r\n";
        let want = ["Mozilla", "Developer Network", ""];

        let frames = FramedRead::new(std::io::Cursor::new(input), ChunkedDecoder::default());
        let got = frames.try_collect::<Vec<Bytes>>().await.unwrap();

        assert_eq!(3, got.len());
        for i in 0..3 {
            assert_eq!(want[i], String::from_utf8_lossy(got[i].chunk()));
        }
    }
}
