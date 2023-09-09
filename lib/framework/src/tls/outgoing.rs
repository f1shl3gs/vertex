use std::io;

use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;

use super::MaybeTls;
use crate::tcp::{self, TcpKeepaliveConfig};

pub type MaybeTlsStream<S> = MaybeTls<S, TlsStream<S>>;

impl MaybeTlsStream<TcpStream> {
    pub fn set_keepalive(&mut self, keepalive: TcpKeepaliveConfig) -> io::Result<()> {
        let stream = match self {
            Self::Raw { raw } => raw,
            Self::Tls { tls } => tls.get_ref().0,
        };

        if let Some(timeout) = keepalive.timeout {
            let config = socket2::TcpKeepalive::new().with_time(timeout);

            tcp::set_keepalive(stream, &config)?;
        }

        Ok(())
    }

    pub fn set_send_buffer_bytes(&mut self, bytes: usize) -> io::Result<()> {
        let stream = match self {
            Self::Raw { raw } => raw,
            Self::Tls { tls } => tls.get_ref().0,
        };

        tcp::set_send_buffer_size(stream, bytes)
    }
}
