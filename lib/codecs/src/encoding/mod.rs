mod format;
mod framing;

use bytes::BytesMut;
use event::Event;
use serde::{Deserialize, Serialize};
use tokio_util::codec::Encoder;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializerConfig {
    Json,
    Logfmt,
    NativeJson,
    Text,
}

impl SerializerConfig {}

pub struct Serializer<T> {
    inner: T,
}

impl<T> Serializer<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> Encoder<Event> for Serializer<T>
where
    T: Encoder<Event>,
    T::Error: From<std::io::Error>,
{
    type Error = crate::Error;

    fn encode(&mut self, item: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // TODO: handle error
        let _n = self.inner.encode(item, dst);
        Ok(())
    }
}
