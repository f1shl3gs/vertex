mod adaptive_concurrency;
mod buffer;
pub mod builder;
mod compressor;
pub mod encoding;
mod request_builder;
pub mod retries;
pub mod service;
mod socket_bytes_sink;
pub mod tcp;
pub mod udp;
pub mod unix;

#[cfg(test)]
pub mod testing;

// re-export
pub use buffer::*;
pub use compressor::*;
pub use encoding::*;
pub use request_builder::RequestBuilder;

use snafu::Snafu;

#[derive(Debug, Snafu)]
enum SinkBuildError {
    #[snafu(display("Missing host in address field"))]
    MissingHost,
    #[snafu(display("Missing port in address field"))]
    MissingPort,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // some features only use some variants
pub enum SocketMode {
    Tcp,
    Udp,
    Unix,
}

impl SocketMode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Unix => "unix",
        }
    }
}
