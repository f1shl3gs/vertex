use serde::{Deserialize, Serialize};
use socket2::SockRef;
use tokio::net::TcpStream;

use crate::config::GenerateConfig;

/// Configuration for keepalive probes in a TCP Stream
///
/// This config's properties map to TCP keepalive properties in Tokio:
/// https://github.com/tokio-rs/tokio/blob/tokio-0.2.22/tokio/src/net/tcp/stream.rs#L516-L537
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TcpKeepaliveConfig {
    #[serde(with = "humanize::duration::serde_option")]
    pub timeout: Option<std::time::Duration>,
}

impl GenerateConfig for TcpKeepaliveConfig {
    fn generate_config() -> String {
        r#"
# The time a connection needs to be idle before sending TCP
# keepalive probes.
#
# timeout: 120s
"#
        .into()
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
