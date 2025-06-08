use std::collections::BTreeMap;
use std::num::ParseIntError;

use bytes::{Bytes, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    UnknownCommand(String),

    #[error("server error {0}")]
    Server(String),

    #[error("parse frame failed, {0}")]
    Parse(String),

    #[error("unknown frame type")]
    UnknownFrameType,
}

#[cfg(test)]
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Error::Io(a), Error::Io(b)) => a.kind() == b.kind(),
            (Error::UnknownCommand(a), Error::UnknownCommand(b)) => a == b,
            (Error::Server(a), Error::Server(b)) => a == b,
            (Error::Parse(a), Error::Parse(b)) => a == b,
            (Error::UnknownFrameType, Error::UnknownFrameType) => true,
            _ => false,
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(err: ParseIntError) -> Self {
        Error::Parse(err.to_string())
    }
}

#[derive(Debug)]
pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> Result<Self, Error> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    pub async fn execute<T>(&mut self, cmds: &[&str]) -> Result<T, Error>
    where
        T: FromFrame,
    {
        let mut buf = BytesMut::with_capacity(256);

        buf.extend_from_slice(format!("*{}\r\n", cmds.len()).as_bytes());
        for cmd in cmds {
            buf.extend_from_slice(format!("${}\r\n{}\r\n", cmd.len(), cmd).as_bytes());
        }
        self.stream.write_all(&buf).await?;

        buf.clear();
        loop {
            if self.stream.read_buf(&mut buf).await? == 0 {
                return Err(Error::Io(std::io::ErrorKind::UnexpectedEof.into()));
            }

            match Frame::parse(&buf) {
                Ok(Some(frame)) => {
                    return match frame {
                        Frame::Error(err) => {
                            if err.starts_with("ERR unknown command") {
                                return Err(Error::UnknownCommand(err.to_string()));
                            }

                            Err(Error::Server(err.to_string()))
                        }
                        _ => T::from_frame(frame),
                    };
                }
                // we need more data
                Ok(None) => continue,
                // something wrong
                Err(err) => return Err(err),
            }
        }
    }
}

/// A RESP2 frame in the Redis protocol.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Clone, Debug)]
pub enum Frame<'a> {
    Simple(&'a str),
    Error(&'a str),
    Integer(i64),
    Bulk(&'a [u8]),
    Null,
    Array(Vec<Frame<'a>>),
}

impl Frame<'_> {
    // Zero copy parser
    //
    // https://redis.io/docs/latest/develop/reference/protocol-spec/
    #[inline]
    pub fn parse(buf: &[u8]) -> Result<Option<Frame>, Error> {
        Frame::parse_with_pos(buf, &mut 0)
    }

    fn parse_with_pos<'a>(buf: &'a [u8], pos: &mut usize) -> Result<Option<Frame<'a>>, Error> {
        // buf is too short,
        // the shortest response looks like `:0\r\n` which is 4 bytes
        if buf.len() - *pos < 1 + 1 + 2 {
            return Ok(None);
        }

        let typ = buf[*pos];
        *pos += 1;

        match typ {
            b'+' => match read_until_crlf(buf, pos) {
                Some((start, end)) => {
                    let s = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };

                    Ok(Some(Frame::Simple(s)))
                }
                None => Ok(None),
            },
            b'-' => match read_until_crlf(buf, pos) {
                Some((start, end)) => {
                    let s = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };

                    Ok(Some(Frame::Error(s)))
                }
                None => Ok(None),
            },
            b':' => match read_until_crlf(buf, pos) {
                Some((start, end)) => {
                    let s = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };
                    let value = s.parse::<i64>()?;

                    Ok(Some(Frame::Integer(value)))
                }
                None => Ok(None),
            },
            b'$' => {
                if b'-' == buf[*pos] {
                    match read_until_crlf(buf, pos) {
                        Some((start, end)) => {
                            if &buf[start..end] != b"-1" {
                                return Err(Error::Server("protocol error".to_string()));
                            }

                            Ok(Some(Frame::Null))
                        }
                        None => Ok(None),
                    }
                } else {
                    // read the bulk string
                    match read_until_crlf(buf, pos) {
                        Some((start, end)) => {
                            let s = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };
                            let value = s.parse::<usize>()?;

                            if buf.len() - *pos < value + 2 {
                                return Ok(None);
                            }

                            let data = &buf[*pos..*pos + value];
                            *pos += value + 2;

                            Ok(Some(Frame::Bulk(data)))
                        }
                        None => Ok(None),
                    }
                }
            }
            b'*' => {
                let len = match read_until_crlf(buf, pos) {
                    Some((start, end)) => {
                        let s = unsafe { std::str::from_utf8_unchecked(&buf[start..end]) };

                        s.parse::<usize>()?
                    }
                    None => return Ok(None),
                };

                let mut frames = Vec::with_capacity(len);
                for _ in 0..len {
                    match Frame::parse_with_pos(buf, pos) {
                        Ok(Some(frame)) => frames.push(frame),
                        Ok(None) => return Ok(None),
                        Err(err) => return Err(err),
                    }
                }

                Ok(Some(Frame::Array(frames)))
            }
            _ => Err(Error::UnknownFrameType),
        }
    }
}

#[inline]
fn read_until_crlf(buf: &[u8], pos: &mut usize) -> Option<(usize, usize)> {
    let start = *pos;
    let len = buf[start..]
        .windows(2)
        .position(|window| window == b"\r\n")?;

    *pos += len + 2;

    Some((start, start + len))
}

pub trait FromFrame: Sized {
    fn from_frame(frame: Frame) -> Result<Self, Error>;
}

impl FromFrame for () {
    fn from_frame(_: Frame) -> Result<(), Error> {
        Ok(())
    }
}

impl FromFrame for i64 {
    fn from_frame(frame: Frame) -> Result<Self, Error> {
        match frame {
            Frame::Integer(value) => Ok(value),
            _ => Err(Error::UnknownFrameType),
        }
    }
}

impl FromFrame for String {
    fn from_frame(frame: Frame) -> Result<Self, Error> {
        match frame {
            Frame::Simple(s) => Ok(s.to_string()),
            Frame::Bulk(b) => String::from_utf8(b.to_vec())
                .map_err(|_err| Error::Server("invalid utf8 bulk".to_string())),
            _ => Err(Error::UnknownFrameType),
        }
    }
}

impl FromFrame for Bytes {
    fn from_frame(frame: Frame) -> Result<Self, Error> {
        match frame {
            Frame::Bulk(b) => Ok(Bytes::from(b.to_vec())),
            _ => Err(Error::UnknownFrameType),
        }
    }
}

impl<T: FromFrame> FromFrame for Vec<T> {
    fn from_frame(frame: Frame) -> Result<Self, Error> {
        match frame {
            Frame::Array(arr) => arr
                .into_iter()
                .map(|item| FromFrame::from_frame(item))
                .collect::<Result<Vec<T>, Error>>(),
            _ => Err(Error::UnknownFrameType),
        }
    }
}

impl FromFrame for BTreeMap<String, String> {
    fn from_frame(frame: Frame) -> Result<Self, Error> {
        match frame {
            Frame::Array(arr) => {
                if arr.len() % 2 != 0 {
                    return Err(Error::Parse(
                        "array length is not divisible by 2".to_string(),
                    ));
                }

                let mut map = BTreeMap::new();
                let mut arr = arr.into_iter();
                loop {
                    let Some(Frame::Bulk(key)) = arr.next() else {
                        break;
                    };

                    let Some(Frame::Bulk(value)) = arr.next() else {
                        break;
                    };

                    let key = unsafe { String::from_utf8_unchecked(key.to_vec()) };
                    let value = unsafe { String::from_utf8_unchecked(value.to_vec()) };

                    map.insert(key, value);
                }

                Ok(map)
            }
            _ => Err(Error::UnknownFrameType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        for (input, expected) in [
            ("+OK\r\n", Ok(Some(Frame::Simple("OK")))),
            ("+OK\r", Ok(None)),
            ("+OK", Ok(None)),
            ("+O", Ok(None)),
            ("+", Ok(None)),
            (
                "-Error message\r\n",
                Ok(Some(Frame::Error("Error message"))),
            ),
            (
                "-ERR unknown command 'asdf'\r\n",
                Ok(Some(Frame::Error("ERR unknown command 'asdf'"))),
            ),
            (
                "-WRONGTYPE Operation against a key holding the wrong kind of value\r\n",
                Ok(Some(Frame::Error(
                    "WRONGTYPE Operation against a key holding the wrong kind of value",
                ))),
            ),
            (":0\r\n", Ok(Some(Frame::Integer(0)))),
            (":1000\r\n", Ok(Some(Frame::Integer(1000)))),
            ("$5\r\nhello\r\n", Ok(Some(Frame::Bulk(b"hello")))),
            ("$0\r\n\r\n", Ok(Some(Frame::Bulk(b"")))),
            ("$-1\r\n", Ok(Some(Frame::Null))),
            ("*0\r\n", Ok(Some(Frame::Array(vec![])))),
            (
                "*2\r\n$5\r\nhello\r\n$5\r\nworld\r\n",
                Ok(Some(Frame::Array(vec![
                    Frame::Bulk(b"hello"),
                    Frame::Bulk(b"world"),
                ]))),
            ),
            ("*2\r\n$5\r\nhello\r\n$5\r\nworld\r", Ok(None)),
            (
                "*3\r\n:1\r\n:2\r\n:3\r\n",
                Ok(Some(Frame::Array(vec![
                    Frame::Integer(1),
                    Frame::Integer(2),
                    Frame::Integer(3),
                ]))),
            ),
            // mixed
            (
                "*5\r\n:1\r\n:2\r\n:3\r\n:4\r\n$5\r\nhello\r\n",
                Ok(Some(Frame::Array(vec![
                    Frame::Integer(1),
                    Frame::Integer(2),
                    Frame::Integer(3),
                    Frame::Integer(4),
                    Frame::Bulk(b"hello"),
                ]))),
            ), // This is a RESP3
               // ("*-1\r\n", Ok(Some(Frame::Array(vec![])))),
        ] {
            let got = Frame::parse_with_pos(input.as_bytes(), &mut 0);
            assert_eq!(got, expected, "incorrect input: {:?}", input);
        }
    }
}
