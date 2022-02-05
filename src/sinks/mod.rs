#[cfg(feature = "sinks-blackhole")]
pub mod blackhole;
#[cfg(feature = "sinks-clickhouse")]
mod clickhouse;
#[cfg(feature = "sinks-elasticsearch")]
mod elasticsearch;
#[cfg(any(feature = "sinks-kafka", feature = "rdkafka"))]
mod kafka;
#[cfg(feature = "sinks-loki")]
pub mod loki;
#[cfg(feature = "sinks-prometheus_exporter")]
mod prometheus_exporter;
#[cfg(feature = "sinks-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sinks-pulsar")]
mod pulsar;
#[cfg(feature = "sinks-socket")]
pub mod socket;
#[cfg(feature = "sinks-stdout")]
mod stdout;
#[cfg(feature = "sinks-vertex")]
mod vertex;

use snafu::Snafu;

/// Common build errors
#[derive(Debug, Snafu)]
pub enum BuildError {
    #[snafu(display("URI parse error: {}", source))]
    UriParse { source: http::uri::InvalidUri },
}
