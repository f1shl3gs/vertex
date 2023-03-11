use std::io::Write;

use bytes::{BufMut, BytesMut};
use codecs::encoding::{EncodingError, SerializeError};
use event::Event;
use jaeger::agent::{serialize_batch, BufferClient, UDP_PACKET_MAX_LENGTH};

#[derive(Clone, Debug)]
pub struct ThriftEncoder {}

impl ThriftEncoder {
    pub const fn new() -> Self {
        Self {}
    }
}

impl tokio_util::codec::Encoder<Event> for ThriftEncoder {
    type Error = EncodingError;

    fn encode(&mut self, event: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // TODO: reuse client
        let mut client = BufferClient::default();
        let trace = event.into_trace();

        match serialize_batch(&mut client, trace.into(), UDP_PACKET_MAX_LENGTH) {
            Ok(data) => {
                dst.writer().write_all(&data)?;

                Ok(())
            }
            Err(err) => {
                warn!(
                    message = "Encode jaeger batch failed",
                    ?err,
                    internal_log_rate_limit = true
                );

                Err(EncodingError::Serialize(SerializeError::Other(err.into())))
            }
        }
    }
}
