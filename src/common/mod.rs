#[cfg(feature = "rdkafka")]
pub mod kafka;

#[cfg(any(
    feature = "sinks-prometheus_remote_write",
    feature = "sources-prometheus_remote_write"
))]
pub mod prometheus;
