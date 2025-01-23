use std::net::SocketAddr;

use codecs::decoding::{DeserializerConfig, FramingConfig};
use codecs::{Decoder, DecodingConfig};
use configurable::Configurable;
use framework::config::{Resource, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;
use value::value;

#[derive(Debug, Clone, Deserialize, Serialize, Configurable)]
pub struct Config {
    address: SocketAddr,

    max_length: usize,

    receive_buffer_size: Option<usize>,

    framing: Option<FramingConfig>,

    decoding: DeserializerConfig,
}

impl Config {
    pub fn resource(&self) -> Resource {
        Resource::udp(self.address)
    }

    pub fn run(&self, cx: SourceContext) -> crate::Result<Source> {
        let decoding = self.decoding.clone();
        let framing = self
            .framing
            .clone()
            .unwrap_or_else(|| decoding.default_stream_framing());
        let decoder = DecodingConfig::new(framing, decoding).build();

        Ok(udp(
            self.address,
            self.receive_buffer_size,
            decoder,
            cx.shutdown,
            cx.output,
        ))
    }
}

pub fn udp(
    address: SocketAddr,
    receive_buffer_size: Option<usize>,
    decoder: Decoder,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Source {
    Box::pin(async move {
        let socket = tokio::net::UdpSocket::bind(&address).await.map_err(|err| {
            warn!(message = "binding udp socket", %err, %address);
        })?;

        if let Some(bytes) = receive_buffer_size {
            if let Err(err) = framework::udp::set_receive_buffer_size(&socket, bytes) {
                warn!(
                    message = "setting receive buffer size failed",
                    %err,
                    %address
                );
            }
        }

        let mut buf = [0u8; u16::MAX as usize];
        loop {
            let (size, peer) = tokio::select! {
                result = socket.recv_from(&mut buf) => match result {
                    Ok(res) => res,
                    Err(_err) => {
                        return Err(())
                    }
                },
                _ = &mut shutdown => return Ok(())
            };

            let mut stream = FramedRead::new(&buf[..size], decoder.clone());
            while let Some(result) = stream.next().await {
                match result {
                    Ok((mut events, _size)) => {
                        let mut metadata = value!({"protocol": "udp"});
                        metadata.insert("host", peer.ip().to_string());
                        metadata.insert("port", peer.port());

                        events.for_each_log(|log| {
                            log.metadata_mut()
                                .value_mut()
                                .insert("socket", metadata.clone());
                        });

                        if let Err(_err) = output.send(events).await {
                            warn!(message = "sending events failed");

                            return Ok(());
                        }
                    }
                    Err(err) => {
                        warn!(
                            message = "receiving udp frame",
                            %err,
                            %peer,
                            internal_log_rate_secs = 10,
                        );

                        break;
                    }
                }
            }
        }
    })
}
