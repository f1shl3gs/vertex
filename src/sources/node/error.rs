use std::num::{ParseFloatError, ParseIntError};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Integer(ParseIntError),
    #[error(transparent)]
    Float(ParseFloatError),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("glob pattern error, {0}")]
    Glob(#[from] glob::PatternError),
    #[error("io error: {err}, {msg}")]
    Io { err: std::io::Error, msg: String },
    #[error(transparent)]
    Parse(ParseError),
    #[error("{0}")]
    Other(String),

    #[error("no data")]
    NoData,
    #[error("malformed {0}")]
    Malformed(&'static str),
}

impl From<&'static str> for Error {
    fn from(value: &'static str) -> Self {
        Error::Other(value.into())
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Error::Other(value)
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Parse(ParseError::Integer(value))
    }
}

impl From<ParseFloatError> for Error {
    fn from(value: ParseFloatError) -> Self {
        Error::Parse(ParseError::Float(value))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io {
            msg: "".to_string(),
            err,
        }
    }
}

impl Error {
    pub fn is_not_found(&self) -> bool {
        match self {
            Error::Io { err, .. } => err.kind() == std::io::ErrorKind::NotFound,
            _ => false,
        }
    }
}
