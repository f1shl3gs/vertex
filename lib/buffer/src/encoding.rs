use std::fmt::Debug;

use bytes::{Buf, BufMut};
use finalize::AddBatchNotifier;

/// An object that can encode and decode itself to and from a buffer.
pub trait Encodable: AddBatchNotifier + Debug + Send + Sized + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Attempts to encode this value into the given buffer.
    ///
    /// # Errors
    ///
    /// If there is an error while attempting to encode this value, an error variant
    /// will be returned describing the error.
    ///
    /// Practically speaking, based on the API, encoding errors should generally only
    /// occur if there is insufficient space in the buffer to fully encode this value.
    /// However, this is not guaranteed.
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), Self::Error>;

    /// Attempts to decode an instance of this type from the given buffer.
    ///
    /// # Errors
    ///
    /// If there is an error while attempting to decode a value from the given
    /// buffer, an error variant will be returned describing the error.
    fn decode<B: Buf>(buf: B) -> Result<Self, Self::Error>;

    /// Get the encoded size, in bytes, of this value if available.,
    fn byte_size(&self) -> usize;
}
