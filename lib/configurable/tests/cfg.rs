#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;

use configurable_derive::Configurable;
use serde::{Deserialize, Serialize};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Mode {
    Tcp {
        /// The address to listen for connections on, or systemd#N to use the Nth
        /// socket passed by systemd socket activation. If an address is used it
        /// must include a port.
        #[configurable(required, format = "ip-address", example = "0.0.0.0:9000")]
        address: SocketAddr,

        /// Configures the receive buffer size using the "SO_RCVBUF" option on the socket.
        #[serde(default)]
        receive_buffer_bytes: Option<usize>,

        /// The max number of TCP connections that will be processed.
        connection_limit: Option<u32>,
    },
    Udp {
        /// The address to listen for connections on, or systemd#N to use the Nth
        /// socket passed by systemd socket activation. If an address is used it
        /// must include a port
        #[configurable(required, format = "ip-address", example = "0.0.0.0:9000")]
        address: SocketAddr,

        /// Configures the recive buffer size using the "SO_RCVBUF" option on the socket.
        #[serde(default)]
        receive_buffer_bytes: Option<usize>,
    },
    #[cfg(unix)]
    Unix {
        /// Unix socket file path.
        #[configurable(required)]
        path: PathBuf,
    },
}

#[test]
fn generate() {}
