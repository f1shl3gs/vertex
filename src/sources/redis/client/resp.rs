use std::io::{self};
use std::num::ParseIntError;

use bytes::Bytes;

use super::frame::{Error as FrameErr, Frame};

#[derive(Debug, thiserror::Error)]
pub enum RespErr {
    #[error("{0}")]
    Server(String),

    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Frame(#[from] FrameErr),
}

impl From<String> for RespErr {
    fn from(s: String) -> Self {
        RespErr::Server(s)
    }
}

pub trait FromRespValue: Sized {
    fn from_frame(frame: Frame) -> Result<Self, RespErr>;
}

impl FromRespValue for i64 {
    fn from_frame(frame: Frame) -> Result<Self, RespErr> {
        match frame {
            Frame::Simple(s) => s
                .parse()
                .map_err(|err: ParseIntError| RespErr::Server(err.to_string())),
            _ => Err(FrameErr::InvalidResponseType.into()),
        }
    }
}

impl<T: FromRespValue> FromRespValue for Vec<T> {
    fn from_frame(frame: Frame) -> Result<Self, RespErr> {
        match frame {
            Frame::Array(arr) => arr
                .into_iter()
                .map(|item| FromRespValue::from_frame(item))
                .collect::<Result<Vec<T>, RespErr>>(),
            _ => Err(FrameErr::InvalidResponseType.into()),
        }
    }
}

impl FromRespValue for Bytes {
    fn from_frame(frame: Frame) -> Result<Self, RespErr> {
        match frame {
            Frame::Bulk(b) => Ok(b),
            _ => Err(FrameErr::InvalidResponseType.into()),
        }
    }
}

impl FromRespValue for String {
    fn from_frame(frame: Frame) -> Result<Self, RespErr> {
        match frame {
            Frame::Simple(s) => Ok(s),
            Frame::Bulk(b) => String::from_utf8(b.to_vec())
                .map_err(|_err| RespErr::Server("invalid utf8 bulk".to_string())),
            _ => Err(FrameErr::InvalidResponseType.into()),
        }
    }
}

impl FromRespValue for u64 {
    fn from_frame(frame: Frame) -> Result<Self, RespErr> {
        match frame {
            Frame::Integer(u) => Ok(u),
            _ => Err(FrameErr::InvalidResponseType.into()),
        }
    }
}
