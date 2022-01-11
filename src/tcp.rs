use serde::{Deserialize, Serialize};
use socket2::SockRef;
use tokio::net::TcpStream;

use crate::config::{deserialize_duration_option, serialize_duration_option};

/// Configuration for keepalive probes in a TCP Stream
///
/// This config's properties map to TCP keepalive properties in Tokio:
/// https://github.com/tokio-rs/tokio/blob/tokio-0.2.22/tokio/src/net/tcp/stream.rs#L516-L537
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TcpKeepaliveConfig {
    #[serde(
        deserialize_with = "deserialize_duration_option",
        serialize_with = "serialize_duration_option"
    )]
    pub timeout: Option<std::time::Duration>,
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
