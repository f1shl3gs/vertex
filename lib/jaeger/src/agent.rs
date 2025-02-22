use std::fmt::{Debug, Formatter};

use prost::bytes::{BufMut, BytesMut};
use thrift::protocol::{
    TBinaryInputProtocol, TBinaryOutputProtocol, TCompactInputProtocol, TCompactOutputProtocol,
    TInputProtocol,
};
use thrift::transport::{ReadHalf, TIoChannel, WriteHalf};

use crate::Batch;
use crate::thrift::agent::{AgentEmitBatchArgs, AgentSyncClient, TAgentSyncClient};
use crate::transport::{TBufferChannel, TNoopChannel};

/// The max size of UDP packet we want to send, synced with jaeger-agent
pub const UDP_PACKET_MAX_LENGTH: usize = 65000;

pub struct BufferClient {
    buffer: ReadHalf<TBufferChannel>,
    client: AgentSyncClient<
        TCompactInputProtocol<TNoopChannel>,
        TCompactOutputProtocol<WriteHalf<TBufferChannel>>,
    >,
}

impl Debug for BufferClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferClient")
            .field("buffer", &self.buffer)
            .field("client", &"AgentSyncClient")
            .finish()
    }
}

impl Default for BufferClient {
    fn default() -> Self {
        let (buffer, write) = TBufferChannel::with_capacity(UDP_PACKET_MAX_LENGTH)
            .split()
            .unwrap();
        let client = AgentSyncClient::new(
            TCompactInputProtocol::new(TNoopChannel),
            TCompactOutputProtocol::new(write),
        );

        Self { buffer, client }
    }
}

pub fn deserialize_compact_batch(input: Vec<u8>) -> thrift::Result<Batch> {
    let reader = std::io::Cursor::new(input);
    let mut input = TCompactInputProtocol::new(reader);
    input.read_message_begin()?;
    let args = AgentEmitBatchArgs::read_from_in_protocol(&mut input)?;
    Ok(args.batch)
}

pub fn deserialize_binary_batch(input: Vec<u8>) -> thrift::Result<Batch> {
    let reader = std::io::Cursor::new(input);
    let mut input = TBinaryInputProtocol::new(reader, false);
    input.read_message_begin()?;
    let args = AgentEmitBatchArgs::read_from_in_protocol(&mut input)?;
    Ok(args.batch)
}

pub fn serialize_binary_batch(batch: Batch) -> thrift::Result<Vec<u8>> {
    let mut buf = BytesMut::new().writer();
    let mut op = TBinaryOutputProtocol::new(&mut buf, false);

    batch.write_to_out_protocol(&mut op)?;

    Ok(buf.into_inner().to_vec())
}

pub fn serialize_batch(
    client: &mut BufferClient,
    batch: Batch,
    max_packet_size: usize,
) -> thrift::Result<Vec<u8>> {
    client.client.emit_batch(batch)?;
    let payload = client.buffer.take_bytes();

    if payload.len() > max_packet_size {
        return Err(thrift::ProtocolError::new(
            thrift::ProtocolErrorKind::SizeLimit,
            format!(
                "jaeger exporter payload size of {} bytes over max UDP packet size of {} bytes.\
                Try setting a smaller batch size or turn auto split on",
                payload.len(),
                max_packet_size
            ),
        )
        .into());
    }

    Ok(payload)
}

/*
fn serialize_batch_vectored(
    client: &mut BufferClient,
    mut batch: jaeger::Batch,
    max_packet_size: usize,
    output: &mut Vec<Vec<u8>>,
) -> thrift::Result<()> {
    client.client.emit_batch(batch.clone())?;
    let payload = client.buffer.take_bytes();

    if payload.len() <= max_packet_size {
        output.push(payload);
        return Ok(());
    }

    if batch.spans.len() <= 1 {
        return Err(thrift::ProtocolError::new(
            thrift::ProtocolErrorKind::SizeLimit,
            format!(
                "single span's jaeger exporter payload size of {} bytes over max UDP packet \
                size of {} bytes",
                payload.len(),
                max_packet_size,
            ),
        )
        .into());
    }

    let mid = batch.spans.len() / 2;
    let new_spans = batch.spans.drain(mid..).collect::<Vec<_>>();
    let new_batch = jaeger::Batch::new(batch.process.clone(), new_spans);

    serialize_batch_vectored(client, batch, max_packet_size, output)?;
    serialize_batch_vectored(client, new_batch, max_packet_size, output)?;

    Ok(())
}
*/
