use configurable::Configurable;
use serde::{Deserialize, Serialize};
use socket2::SockRef;
use tokio::net::TcpStream;

/// Configuration for keepalive probes in a TCP Stream
///
/// This config's properties map to TCP keepalive properties in Tokio:
/// https://github.com/tokio-rs/tokio/blob/tokio-0.2.22/tokio/src/net/tcp/stream.rs#L516-L537
#[derive(Configurable, Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TcpKeepaliveConfig {
    /// The time a connection needs to be idle before sending TCP
    /// keepalive probes.
    #[serde(with = "humanize::duration::serde_option")]
    pub timeout: Option<std::time::Duration>,
}

impl TcpKeepaliveConfig {
    pub fn apply_to(&self, stream: &TcpStream) -> std::io::Result<()> {
        let Some(timeout) = self.timeout else {
            return Ok(());
        };

        let config = socket2::TcpKeepalive::new().with_time(timeout);

        set_keepalive(stream, &config)
    }
}

// This function will be obsolete after tokio/mio internally use `socket2` and expose the
// methods to apply options to a socket.
pub fn set_keepalive(socket: &TcpStream, params: &socket2::TcpKeepalive) -> std::io::Result<()> {
    SockRef::from(socket).set_tcp_keepalive(params)
}

// This function will be obsolete after tokio/mio internally use `socket2` and expose the methods to
// apply options to a socket.
pub fn set_receive_buffer_size(socket: &TcpStream, size: usize) -> std::io::Result<()> {
    SockRef::from(socket).set_recv_buffer_size(size)
}

// This function will be obsolete after tokio/mio internally use `socket2` and expose the methods to
// apply options to a socket.
pub fn set_send_buffer_size(socket: &TcpStream, size: usize) -> std::io::Result<()> {
    SockRef::from(socket).set_send_buffer_size(size)
}
