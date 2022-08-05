use tokio_util::codec::LinesCodecError;

/// An error that occurred while encoding/decoding structured events to/from a byte stream /
/// byte messages.
#[derive(Debug)]
pub enum FramingError {
    /// Io error
    Io(std::io::Error),

    /// LinesCodecError
    LinesCodec(LinesCodecError),
}

impl From<std::io::Error> for FramingError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<LinesCodecError> for FramingError {
    fn from(err: LinesCodecError) -> Self {
        Self::LinesCodec(err)
    }
}
