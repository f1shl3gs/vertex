#[cfg(feature = "sinks-blackhole")]
pub mod blackhole;
#[cfg(feature = "sinks-clickhouse")]
mod clickhouse;
#[cfg(feature = "sinks-console")]
mod console;
#[cfg(feature = "sinks-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sinks-http")]
mod http;
#[cfg(feature = "sinks-influxdb")]
mod influxdb;
#[cfg(feature = "sinks-jaeger")]
mod jaeger;
#[cfg(feature = "sinks-kafka")]
mod kafka;
#[cfg(feature = "sinks-loki")]
pub mod loki;
#[cfg(feature = "sinks-prometheus_exporter")]
mod prometheus_exporter;
#[cfg(feature = "sinks-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sinks-skywalking")]
mod skywalking;
#[cfg(feature = "sinks-socket")]
pub mod socket;
