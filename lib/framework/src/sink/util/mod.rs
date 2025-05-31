mod adaptive_concurrency;
pub mod buffer;
pub mod builder;
mod compressor;
pub mod encoding;
mod partitioner;
mod request_builder;
pub mod retries;
pub mod service;
pub mod sink;
mod snappy;
mod zstd;

#[cfg(any(test, feature = "test-util"))]
pub mod testing;

pub use buffer::{
    Buffer, Compression,
    metrics::{MetricNormalize, MetricNormalizer, MetricSet, MetricsBuffer},
    partition::{Partition, PartitionBuffer, PartitionInnerBuffer},
    vec::{EncodedLength, VecBuffer},
};
pub use compressor::Compressor;
pub use encoding::*;
pub use partitioner::KeyPartitioner;
pub use request_builder::{EncodeResult, RequestBuilder, RequestMetadata};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SinkBuildError {
    #[error("Missing host in address field")]
    MissingHost,
    #[error("Missing port in address field")]
    MissingPort,
}

#[derive(Debug, Clone, Copy)]
pub enum SocketMode {
    Tcp,
    Udp,
    Unix,
}

impl SocketMode {
    pub const fn as_str(self) -> &'static str {
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
