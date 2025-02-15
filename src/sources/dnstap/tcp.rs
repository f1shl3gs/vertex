use std::net::SocketAddr;

use configurable::Configurable;
use framework::config::{Resource, SourceContext};
use framework::tcp::TcpKeepaliveConfig;
use framework::tls::{MaybeTlsListener, TlsConfig};
use framework::Source;
use serde::{Deserialize, Serialize};

use super::serve_conn;

#[derive(Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    address: SocketAddr,

    tls: Option<TlsConfig>,

    keepalive: Option<TcpKeepaliveConfig>,

    /// The size of the receive buffer used for each connection.
    #[serde(default, with = "humanize::bytes::serde_option")]
    receive_buffer_bytes: Option<usize>,
    // /// The timeout before a connection is forcefully closed during shutdown.
    // #[serde(
    //     default = "default_shutdown_timeout",
    //     with = "humanize::duration::serde"
    // )]
    // shutdown_timeout: Duration,
}

impl Config {
    pub async fn build(&self, max_frame_length: usize, cx: SourceContext) -> crate::Result<Source> {
        let mut listener = MaybeTlsListener::bind(&self.address, self.tls.as_ref()).await?;
        let mut shutdown = cx.shutdown;
        let output = cx.output;
        let keepalive = self.keepalive;
        let receive_buffer_bytes = self.receive_buffer_bytes;

        Ok(Box::pin(async move {
            loop {
                let mut stream = tokio::select! {
                    result = listener.accept() => match result {
                        Ok(stream) => stream,
                        Err(err) => {
                            warn!(
                                message = "tcp listener accept error: {}",
                                ?err
                            );

                            continue;
                        }
                    },
                    _ = &mut shutdown => break,
                };

                debug!(message = "accept new connection", peer = ?stream.peer_addr());

                if let Some(keepalive) = &keepalive {
                    if let Err(err) = stream.set_keepalive(keepalive) {
                        warn!(
                            message = "setting TCP keepalive failed",
                            ?err,
                            internal_log_rate_secs = 30,
                        );
                    }
                }

                if let Some(receive_buffer_bytes) = receive_buffer_bytes {
                    if let Err(err) = stream.set_receive_buffer_bytes(receive_buffer_bytes) {
                        warn!(
                            message = "setting receive buffer bytes failed",
                            ?err,
                            internal_log_rate_secs = 30,
                        );
                    }
                }

                let shutdown = shutdown.clone();
                let output = output.clone();
                tokio::spawn(serve_conn(stream, true, max_frame_length, shutdown, output));
            }

            Ok(())
        }))
    }

    pub fn resource(&self) -> Resource {
        Resource::tcp(self.address)
    }

    #[cfg(all(test, feature = "dnstap-integration-tests"))]
    pub fn simple(address: SocketAddr) -> Self {
        Self {
            address,
            tls: None,
            keepalive: None,
            receive_buffer_bytes: None,
        }
    }
}
