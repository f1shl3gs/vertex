//! A collection of codecs that can be used to transform between bytes streams /
//! byte messages, byte frames and structured events.

#![deny(missing_docs)]
#![deny(warnings)]

pub mod decoding;
pub mod encoding;
mod error;
mod ready_frames;

pub use decoding::{Decoder, DecodingConfig};
pub use encoding::{Encoder, EncodingConfig, EncodingConfigWithFraming};
pub use error::FramingError;
pub use ready_frames::ReadyFrames;
