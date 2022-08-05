pub mod json;
pub mod logfmt;
pub mod native_json;
pub mod text;

use std::fmt::{Debug, Formatter};

use dyn_clone::DynClone;
use event::Event;

use crate::encoding::SerializeError;

/// Serialize a structured event into a byte frame.
pub trait Serializer:
    DynClone + Debug + Send + Sync + tokio_util::codec::Encoder<Event, Error = SerializeError>
{
}

dyn_clone::clone_trait_object!(Serializer);

impl<E> Serializer for E where
    E: DynClone
        + Debug
        + Sized
        + Send
        + Sync
        + tokio_util::codec::Encoder<Event, Error = SerializeError>
{
}
