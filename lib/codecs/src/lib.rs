//! A collection of codecs that can be used to transform between bytes streams /
//! byte messages, byte frames and structured events.

#![deny(missing_docs)]
#![deny(warnings)]

pub mod decoding;
pub mod encoding;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
