#[cfg(feature = "sinks-blackhole")]
pub mod blackhole;
#[cfg(feature = "sinks-clickhouse")]
mod clickhouse;
#[cfg(feature = "sinks-console")]
mod console;
#[cfg(feature = "sinks-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sinks-jaeger")]
mod jaeger;
#[cfg(any(feature = "sinks-kafka", feature = "rdkafka"))]
mod kafka;
#[cfg(feature = "sinks-loki")]
pub mod loki;
#[cfg(feature = "sinks-prometheus_exporter")]
mod prometheus_exporter;
#[cfg(feature = "sinks-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sinks-socket")]
pub mod socket;

use thiserror::Error;

/// Common build errors
#[derive(Debug, Error)]
pub enum BuildError {
    #[error("URI parse error: {0}")]
    UriParse(http::uri::InvalidUri),
}
