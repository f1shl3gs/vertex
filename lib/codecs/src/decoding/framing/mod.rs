mod bytes;
mod character;
mod newline;
mod octet_counting;

use ::bytes::{Bytes, BytesMut};
use event::Event;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt::Debug;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

pub use self::bytes::BytesDeserializerConfig;
use crate::FramingError;
pub use character::{CharacterDelimitedDecoder, CharacterDelimitedDecoderConfig};
pub use newline::NewlineDelimitedDecoder;
pub use octet_counting::OctetCountingDecoder;

/// Produce byte frames from a byte stream / byte message.
pub trait Framer:
    tokio_util::codec::Decoder<Item = Bytes, Error = FramingError> + Clone + Debug + Send + Sync
{
}
