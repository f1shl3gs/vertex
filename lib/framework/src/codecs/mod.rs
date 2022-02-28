pub mod decoding;
pub mod encoding;
mod ready_frames;

// re-export
pub use decoding::Decoder;
pub use decoding::{
    BytesDecoderConfig, BytesDeserializerConfig, NewlineDelimitedDecoderConfig,
    OctetCountingDecoder, SyslogDeserializer,
};
pub use ready_frames::ReadyFrames;

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
