use std::net::SocketAddr;

use codecs::Decoder;
use codecs::decoding::{BytesDecoder, SyslogDeserializer};
use configurable::Configurable;
use framework::config::{Resource, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;
use value::OwnedValuePath;

use super::handle_events;

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The address to listen for connections on, or systemd#N to use the Nth
    /// socket passed by systemd socket activation. If an address is used it
    /// must include a port
    #[configurable(format = "ip-address", example = "0.0.0.0:9000")]
    listen: SocketAddr,

    /// Configures the recive buffer size using the "SO_RCVBUF" option on the socket.
    #[serde(default, with = "humanize::bytes::serde_option")]
    receive_buffer_bytes: Option<usize>,
}

impl Config {
    pub fn resource(&self) -> Resource {
        Resource::udp(self.listen)
    }

    pub fn build(
        &self,
        cx: SourceContext,
        max_length: usize,
        host_key: OwnedValuePath,
    ) -> crate::Result<Source> {
        Ok(udp(
            self.listen,
            max_length,
            host_key,
            self.receive_buffer_bytes,
            cx.shutdown,
            cx.output,
        ))
    }
}

fn udp(
    addr: SocketAddr,
    _max_length: usize,
    host_key: OwnedValuePath,
    receive_buffer_bytes: Option<usize>,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Source {
    Box::pin(async move {
        let socket = UdpSocket::bind(&addr)
            .await
            .expect("Failed to bind to UDP listener socket");

        if let Some(receive_buffer_bytes) = receive_buffer_bytes
            && let Err(err) = framework::udp::set_receive_buffer_size(&socket, receive_buffer_bytes)
        {
            warn!(
                message = "Failed configure receive buffer size on UDP socket",
                %err
            );
        }

        info!(
            message = "listening",
            %addr,
            r#type = "udp"
        );

        let mut stream = UdpFramed::new(
            socket,
            Decoder::new(
                BytesDecoder::new().into(),
                SyslogDeserializer::default().into(),
            ),
        )
        .take_until(shutdown)
        .filter_map(|frame| {
            let host_key = host_key.clone();

            async move {
                match frame {
                    Ok(((mut events, _byte_size), received_from)) => {
                        handle_events(&mut events, &host_key, Some(received_from));
                        Some(events)
                    }
                    Err(err) => {
                        warn!(
                            message = "Error reading datagram",
                            %err,
                            internal_log_rate_limit = true
                        );

                        None
                    }
                }
            }
        })
        .boxed();

        match output.send_stream(&mut stream).await {
            Ok(()) => {
                info!(message = "Finished sending");
                Ok(())
            }
            Err(err) => {
                error!(
                    message = "Error sending line",
                    %err
                );

                Err(())
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_udp() {
        let config: Config = serde_yaml::from_str("address: 127.0.0.1:12345").unwrap();

        assert_eq!(config.listen.to_string(), "127.0.0.1:12345");
    }
}
