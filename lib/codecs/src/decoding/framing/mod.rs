mod bytes;
mod character;
mod newline;
mod octet_counting;

use std::fmt::Debug;

pub use self::bytes::BytesDeserializerConfig;
use ::bytes::Bytes;
pub use character::{CharacterDelimitedDecoder, CharacterDelimitedDecoderConfig};
pub use newline::{NewlineDelimitedDecoder, NewlineDelimitedDecoderConfig};
pub use octet_counting::{OctetCountingDecoder, OctetCountingDecoderConfig};

use super::FramingError;

/// Produce byte frames from a byte stream / byte message.
pub trait Framer:
    tokio_util::codec::Decoder<Item = Bytes, Error = FramingError> + Clone + Debug + Send + Sync
{
}
