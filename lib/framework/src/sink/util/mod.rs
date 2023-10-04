mod adaptive_concurrency;
pub mod buffer;
pub mod builder;
mod compressor;
pub mod encoding;
pub mod http;
mod partitioner;
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

pub use buffer::{
    metrics::{MetricNormalize, MetricNormalizer, MetricSet, MetricsBuffer},
    partition::{Partition, PartitionBuffer, PartitionInnerBuffer},
    vec::{EncodedLength, VecBuffer},
    Buffer, Compression,
};
pub use compressor::Compressor;
pub use encoding::*;
pub use partitioner::KeyPartitioner;
pub use request_builder::{EncodeResult, RequestBuilder, RequestMetadata};

use thiserror::Error;

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
