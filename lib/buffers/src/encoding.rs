//! This module defines traits that allow conversion to and from `bytes`
//! buffers. The vector project needs ser/de that is defined by the type being
//! serialized. That is, while it's typical in the ecosystem to define ser/de in
//! terms of `serde` we have protobuf ser/de in places that are not suitable for
//! use with that technique, see [this
//! discussion](https://github.com/danburkert/prost#faq) for details. But, we
//! want generic structures that have type constraints for ser/de and so that's
//! what this module provides. The definition is inspired by the types from
//! `prost::Message`, though split into an encode and decode side as serde
//! does.
use std::error;

use bytes::{Buf, BufMut};

/// Encode a type into a `bytes` buffer, possibly unsuccessfully
pub trait EncodeBytes: Sized {
    /// The type returned when `encode` fails
    type Error: error::Error + Send + Sync + 'static;

    /// Attempt to encode into `B` buffer
    ///
    /// # Errors
    ///
    /// Function will fail when encoding is not possible for the type instance.
    fn encode<B: BufMut>(self, buffer: &mut B) -> Result<(), Self::Error>;

    /// Return the encoded byte size
    ///
    /// For some types it is not clear ahead of time how large the encoded size
    /// will be. For such types the return will be `None`, otherwise `Some`.
    fn encoded_size(&self) -> Option<usize> {
        None
    }
}

/// Decode a type from a `bytes` buffer, possibly unsuccessfully
pub trait DecodeBytes: Sized {
    /// The type returned when `decode` fails
    type Error: error::Error + Send + Sync + 'static;

    /// Attempt to decode from `B` buffer
    ///
    /// # Errors
    ///
    /// Function will fail when decoding is not possible from the passed buffer.
    fn decode<B: Buf>(buffer: B) -> Result<Self, Self::Error>;
}
