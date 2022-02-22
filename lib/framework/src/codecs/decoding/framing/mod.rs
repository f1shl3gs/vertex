use std::fmt::Debug;

use ::bytes::Bytes;
use dyn_clone::DynClone;
use tokio_util::codec::LinesCodecError;

use crate::codecs::StreamDecodingError;

pub mod bytes;
pub mod character;
pub mod newline;
pub mod octet_counting;

/// An error that occurred while producing byte frames from a byte stream byte
/// message.
///
/// It requires conformance to `TcpError` so that we can determine whether the
/// error is recoverable or if trying to continue will lead to hanging up the
/// TCP source indefinitely.
pub trait FramingError: std::error::Error + StreamDecodingError + Send + Sync {}

impl std::error::Error for BoxedFramingError {}

impl FramingError for std::io::Error {}

impl FramingError for LinesCodecError {}

impl From<std::io::Error> for BoxedFramingError {
    fn from(err: std::io::Error) -> Self {
        Box::new(err)
    }
}

impl From<LinesCodecError> for BoxedFramingError {
    fn from(err: LinesCodecError) -> Self {
        Box::new(err)
    }
}

/// A `Box` containing a `FramingError`
pub type BoxedFramingError = Box<dyn FramingError>;

impl StreamDecodingError for BoxedFramingError {
    fn can_continue(&self) -> bool {
        self.as_ref().can_continue()
    }
}

/// Produce byte frames from a byte stream / byte message.
pub trait Framer:
    tokio_util::codec::Decoder<Item = Bytes, Error = BoxedFramingError> + DynClone + Debug + Send + Sync
{
}

/// Default implementation for `Framer`s that implement
/// `tokio_util::codec::Decoder`.
impl<Decoder> Framer for Decoder where
    Decoder: tokio_util::codec::Decoder<Item = Bytes, Error = BoxedFramingError>
        + Clone
        + Debug
        + Send
        + Sync
{
}

dyn_clone::clone_trait_object!(Framer);

/// A `Box` containing a `Framer`.
pub type BoxedFramer = Box<dyn Framer>;

/// Define options for a framer and build it from the config object.
///
/// Implementors must annotate the struct with `#[typetag::serde(name = "...")]`
/// to define which value should be read from the `method` key to select their
/// implementation.
#[typetag::serde(tag = "method")]
pub trait FramingConfig: Debug + DynClone + Send + Sync {
    /// Builds a framer from this configuration.
    ///
    /// Fails if the configuration is invalid.
    fn build(&self) -> crate::Result<BoxedFramer>;
}

dyn_clone::clone_trait_object!(FramingConfig);
