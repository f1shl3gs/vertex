use socket2::SockRef;
use tokio::net::UdpSocket;

// NOTE: this feature is already merge into tokio's master branch,
// See: https://github.com/tokio-rs/tokio/pull/4363

// This function will be obsolete after tokio/mio internally use `socket2` and expose the
// methods to apply options to a socket
pub fn set_receive_buffer_size(socket: &UdpSocket, size: usize) -> std::io::Result<()> {
    SockRef::from(socket).set_recv_buffer_size(size)
}

// This function will be obsolete after tokio/mio internally use `socket2` and expose the
// methods to apply options to a socket
pub fn set_send_buffer_size(socket: &UdpSocket, size: usize) -> std::io::Result<()> {
    SockRef::from(socket).set_send_buffer_size(size)
}
