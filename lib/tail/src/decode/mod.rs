mod bytes;
mod newline;

pub use bytes::BytesDelimitDecoder;
pub use newline::NewlineDecoder;

use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),

    /// The maximum line length was exceeded.
    MaxLengthExceeded,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MaxLengthExceeded => f.write_str("Maximum length exceeded"),
            Error::Io(err) => err.fmt(f),
        }
    }
}
