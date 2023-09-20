use std::fmt::{Display, Formatter};

use super::{DeserializeError, FramingError};

/// An error that occurs while decoding a stream
pub trait StreamDecodingError {
    /// Whether it is reasonable to assume that continuing to read from the
    /// stream in which this error occurred will not result in an indefinite
    /// hang up.
    ///
    /// This can occur e.g. when reading the header of a length-delimited codec
    /// failed and it can no longer be determined where the next header starts
    fn can_continue(&self) -> bool;
}

/// An error that occurred while decoding structured events from a byte stream /
/// byte message.
#[derive(Debug)]
pub enum DecodeError {
    /// Decoder Error need this.
    Io(std::io::Error),

    /// The error occurred while producing byte frames from the byte stream /
    /// byte message.
    Framing(FramingError),

    /// The error occurred while deserialize frame.
    Deserialize(DeserializeError),
}

impl StreamDecodingError for DecodeError {
    fn can_continue(&self) -> bool {
        true
    }
}

impl From<std::io::Error> for DecodeError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::Io(err) => write!(f, "io error {:?}", err),
            DecodeError::Framing(err) => write!(f, "framing error {:?}", err),
            DecodeError::Deserialize(err) => write!(f, "deserialize error {:?}", err),
        }
    }
}
