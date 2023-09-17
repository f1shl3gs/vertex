use std::net::SocketAddr;

use bytes::BytesMut;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use jaeger::Batch;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;

// See https://www.jaegertracing.io/docs/1.31/getting-started/
const PROTOCOL_COMPACT_THRIFT_OVER_UDP_PORT: u16 = 6831;
const PROTOCOL_BINARY_THRIFT_OVER_UDP_PORT: u16 = 6832;

const fn default_max_packet_size() -> usize {
    65000
}

fn default_thrift_compact_socketaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), PROTOCOL_COMPACT_THRIFT_OVER_UDP_PORT)
}

fn default_thrift_binary_socketaddr() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), PROTOCOL_BINARY_THRIFT_OVER_UDP_PORT)
}

/// The Agent can only receive spans over UDP in Thrift format.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-udp-stable
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThriftCompactConfig {
    #[serde(default = "default_thrift_compact_socketaddr")]
    #[configurable(required)]
    pub endpoint: SocketAddr,

    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,

    #[serde(default)]
    pub socket_buffer_size: Option<usize>,
}

/// Most Jaeger Clients use Thrift’s compact encoding, however some client libraries
/// do not support it (notably, Node.js) and use Thrift’s binary encoding.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-udp-stable
#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThriftBinaryConfig {
    #[serde(default = "default_thrift_binary_socketaddr")]
    #[configurable(required)]
    pub endpoint: SocketAddr,

    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,

    #[serde(default)]
    pub socket_buffer_size: Option<usize>,
}

pub(super) async fn serve(
    source: String,
    address: SocketAddr,
    max_packet_size: usize,
    receive_buffer_size: Option<usize>,
    shutdown: ShutdownSignal,
    decode: impl Fn(Vec<u8>) -> std::io::Result<Batch> + Send + Sync + 'static,
    mut output: Pipeline,
) -> framework::Result<()> {
    let socket = UdpSocket::bind(address)
        .await
        .expect("Failed to bind to udp listener socket");

    if let Some(receive_buffer_size) = receive_buffer_size {
        if let Err(err) = framework::udp::set_receive_buffer_size(&socket, receive_buffer_size) {
            warn!(
                message = "Failed configuring receive buffer size on UDP socket",
                %err
            );
        }
    }

    let max_length = if let Some(receive_buffer_size) = receive_buffer_size {
        std::cmp::min(max_packet_size, receive_buffer_size)
    } else {
        max_packet_size
    };

    info!(message = "Listening", %address);

    let mut buf = BytesMut::with_capacity(max_length);
    let recv_bytes = metrics::register_counter("socket_recv_bytes_total", "")
        .recorder([("source", source.into())]);
    loop {
        buf.resize(max_length, 0);

        tokio::select! {
            recv = socket.recv_from(&mut buf) => {
                match recv {
                    Ok((size, _orgin_address)) => {
                        let payload = buf.split_to(size);
                        recv_bytes.inc(size as u64);

                        match decode(payload.to_vec()) {
                            Ok(batch) => {
                                if let Err(err) = output.send(batch).await {
                                    error!(message = "Error sending trace", ?err);

                                    return Err(err.into());
                                }
                            }
                            Err(err) => {
                                warn!(
                                    message = "Decoding batch failed",
                                    ?err,
                                    internal_log_rate_limit = true
                                );
                            }
                        }
                    }
                    Err(err) => {
                        warn!(message = "Receiving udp packet failed", ?err);
                    }
                }
            },

            _ = shutdown.clone() => return Ok(())
        }
    }
}
