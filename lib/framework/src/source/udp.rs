use std::net::SocketAddr;

use event::Events;
use tokio::net::UdpSocket;

use crate::config::SourceContext;
use crate::{Source, udp};

pub trait UdpSource: Sized + Send + Sync + 'static {
    fn build_events(&self, peer: SocketAddr, data: &[u8]) -> Result<Events, crate::Error>;

    fn run(
        self,
        listen: SocketAddr,
        receive_buffer_bytes: Option<usize>,
        cx: SourceContext,
    ) -> crate::Result<Source> {
        let SourceContext {
            mut shutdown,
            mut output,
            ..
        } = cx;

        Ok(Box::pin(async move {
            let socket = match UdpSocket::bind(listen).await {
                Ok(socket) => socket,
                Err(err) => {
                    error!(
                        message = "bind UDP failed",
                        %listen,
                        %err
                    );
                    return Err(());
                }
            };

            if let Some(receive_buffer_bytes) = receive_buffer_bytes
                && let Err(err) = udp::set_receive_buffer_size(&socket, receive_buffer_bytes)
            {
                warn!(
                    message = "failed configure receive buffer size on UDP socket",
                    %listen,
                    %err
                );
            }

            let mut buf = [0u8; u16::MAX as usize];
            loop {
                let (size, peer) = tokio::select! {
                    _ = &mut shutdown => {
                        break
                    },

                    result = socket.recv_from(&mut buf) => match result {
                        Ok(pair) => pair,
                        Err(err) => {
                            warn!(
                                message = "receive UDP socket error",
                                %err,
                                internal_log_rate_secs = 10
                            );

                            continue
                        }
                    }
                };

                match self.build_events(peer, &buf[..size]) {
                    Ok(events) => {
                        if let Err(err) = output.send(events).await {
                            error!(
                                message = "send events to output failed",
                                %err,
                            );

                            break;
                        }
                    }
                    Err(err) => {
                        warn!(
                            message = "build event failed",
                            %peer,
                            %err,
                            internal_log_rate_secs = 10
                        );
                    }
                }
            }

            Ok(())
        }))
    }
}
