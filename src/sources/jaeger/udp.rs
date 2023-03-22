use std::net::SocketAddr;

use bytes::BytesMut;
use event::Event;
use framework::{Pipeline, ShutdownSignal};
use futures_util::future::Shared;
use jaeger::Batch;
use tokio::net::UdpSocket;

pub(super) async fn serve(
    source: String,
    address: SocketAddr,
    max_packet_size: usize,
    receive_buffer_size: Option<usize>,
    shutdown: Shared<ShutdownSignal>,
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
                                if let Err(err) = output.send(Event::from(batch)).await {
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
