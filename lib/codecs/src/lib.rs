//! A collection of codecs that can be used to transform between bytes streams /
//! byte messages, byte frames and structured events.

#![deny(missing_docs)]
// #![deny(warnings)]
#![allow(warnings)]

pub mod decoding;
pub mod encoding;
mod error;

pub use error::FramingError;

/// Basic error type, dynamically dispatched and safe to send across threads.
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
