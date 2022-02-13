use std::fmt::{Debug, Formatter};
use thrift::protocol::{TCompactInputProtocol, TCompactOutputProtocol};
use thrift::transport::{ReadHalf, TIoChannel, WriteHalf};
use tokio::net::{ToSocketAddrs, UdpSocket};

use crate::thrift::agent::TAgentSyncClient;
use crate::thrift::{agent, jaeger};
use crate::transport::{TBufferChannel, TNoopChannel};

/// The max size of UDP packet we want to send, synced with jaeger-agent
pub const UDP_PACKET_MAX_LENGTH: usize = 65000;

pub struct BufferClient {
    buffer: ReadHalf<TBufferChannel>,
    client: agent::AgentSyncClient<
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

impl BufferClient {
    pub fn new() -> Self {
        let (buffer, write) = TBufferChannel::with_capacity(UDP_PACKET_MAX_LENGTH)
            .split()
            .unwrap();
        let client = agent::AgentSyncClient::new(
            TCompactInputProtocol::new(TNoopChannel),
            TCompactOutputProtocol::new(write),
        );

        Self { buffer, client }
    }
}

/// `AgentAsyncClientUdp` implements an async version of the `TAgentSyncClient`
/// trait over UDP.
#[derive(Debug)]
pub struct AgentAsyncClientUdp {
    conn: UdpSocket,
    buffer_client: BufferClient,
    max_packet_size: usize,
    auto_split: bool,
}

impl AgentAsyncClientUdp {
    /// Create a new UDP agent cilent
    pub async fn new<T>(
        host_port: T,
        max_packet_size: Option<usize>,
        auto_split: bool,
    ) -> thrift::Result<Self>
    where
        T: ToSocketAddrs,
    {
        let max_packet_size = max_packet_size.unwrap_or(UDP_PACKET_MAX_LENGTH);
        let (buffer, write) = TBufferChannel::with_capacity(max_packet_size).split()?;
        let client = agent::AgentSyncClient::new(
            TCompactInputProtocol::new(TNoopChannel),
            TCompactOutputProtocol::new(write),
        );

        let conn = tokio::net::UdpSocket::bind("0.0.0.0:0").await?;
        conn.connect(host_port).await?;

        Ok(Self {
            conn,
            buffer_client: BufferClient { buffer, client },
            max_packet_size,
            auto_split,
        })
    }

    /// Emit standard Jaeger Batch
    pub async fn emit_batch(&mut self, batch: jaeger::Batch) -> thrift::Result<()> {
        if !self.auto_split {
            let payload = serialize_batch(&mut self.buffer_client, batch, self.max_packet_size)?;
            self.conn.send(&payload).await?;
            return Ok(());
        }

        let mut buffers = vec![];
        serialize_batch_vectored(
            &mut self.buffer_client,
            batch,
            self.max_packet_size,
            &mut buffers,
        )?;

        for payload in buffers {
            self.conn.send(&payload).await?;
        }

        Ok(())
    }
}

pub fn serialize_batch(
    client: &mut BufferClient,
    batch: jaeger::Batch,
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
