pub mod events;
#[cfg(feature = "rdkafka")]
pub mod kafka;
mod open;
#[cfg(any(
    feature = "sinks-prometheus_remote_write",
    feature = "sources-prometheus_remote_write"
))]
pub mod prometheus;

pub use open::OpenGauge;
