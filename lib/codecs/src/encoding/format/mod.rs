pub mod json;
pub mod logfmt;
pub mod native_json;
pub mod text;

use std::fmt::{Debug, Formatter};

use event::Event;

use crate::encoding::SerializeError;

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
