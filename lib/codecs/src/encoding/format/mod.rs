pub mod json;
pub mod logfmt;
pub mod native_json;
pub mod text;

use std::fmt::Debug;

use event::Event;

use super::SerializeError;

/// Serialize a structured event into a byte frame.
pub trait Serializer:
    Clone + Debug + Send + Sync + tokio_util::codec::Encoder<Event, Error = SerializeError>
{
}

impl<E> Serializer for E where
    E: Clone
        + Debug
        + Sized
        + Send
        + Sync
        + tokio_util::codec::Encoder<Event, Error = SerializeError>
{
}
