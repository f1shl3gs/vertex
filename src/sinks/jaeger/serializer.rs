use bytes::{BufMut, BytesMut};
use codecs::encoding::SerializeError;
use event::Event;
use jaeger::agent::{serialize_batch, BufferClient, UDP_PACKET_MAX_LENGTH};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct ThriftSerializer {}

impl ThriftSerializer {
    pub const fn new() -> Self {
        Self {}
    }
}

impl tokio_util::codec::Encoder<Event> for ThriftSerializer {
    type Error = SerializeError;

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
                    internal_log_rate_secs = 10
                );

                Err(SerializeError::Other(err.into()))
            }
        }
    }
}
