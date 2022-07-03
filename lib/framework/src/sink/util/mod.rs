mod adaptive_concurrency;
pub mod buffer;
pub mod builder;
mod compressor;
pub mod encoding;
pub mod http;
mod request_builder;
pub mod retries;
pub mod service;
pub mod sink;
mod socket_bytes_sink;
pub mod tcp;
pub mod udp;
#[cfg(unix)]
pub mod unix;

#[cfg(any(test, feature = "test-util"))]
pub mod testing;

// re-export
pub use buffer::*;
pub use compressor::*;
pub use encoding::*;
pub use request_builder::RequestBuilder;

use bytes::Bytes;
use event::Event;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::util::encoding::{EncodingConfig, EncodingConfiguration};

#[derive(Debug, Error)]
enum SinkBuildError {
    #[error("Missing host in address field")]
    MissingHost,
    #[error("Missing port in address field")]
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
    #[allow(dead_code)]
    const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Unix => "unix",
        }
    }
}

/// Marker trait for types that can hold a batch of events
pub trait ElementCount {
    fn element_count(&self) -> usize;
}

impl<T> ElementCount for Vec<T> {
    fn element_count(&self) -> usize {
        self.len()
    }
}

impl ElementCount for serde_json::Value {
    fn element_count(&self) -> usize {
        1
    }
}

/**
 * Enum representing different ways to encode events as they are sent into a Sink.
 */
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Text,
    Json,
}

/**
 * Encodes the given event into raw bytes that can be sent into a Sink, according to
 * the given encoding. If there are any errors encoding the event, logs a warning
 * and returns None.
 **/
pub fn encode_log(mut event: Event, encoding: &EncodingConfig<Encoding>) -> Option<Bytes> {
    encoding.apply_rules(&mut event);
    let log = event.into_log();

    let b = match encoding.codec() {
        Encoding::Json => serde_json::to_vec(&log),
        Encoding::Text => {
            let bytes = log
                .get_field(log_schema().message_key())
                .map(|v| v.as_bytes().to_vec())
                .unwrap_or_default();
            Ok(bytes)
        }
    };

    b.map(|mut b| {
        b.push(b'\n');
        Bytes::from(b)
    })
    .map_err(|error| error!(message = "Unable to encode.", %error))
    .ok()
}
