#[cfg(feature = "sources-bind")]
mod bind;
mod blackbox;
mod chrony;
#[cfg(feature = "sources-consul")]
mod consul;
#[cfg(feature = "sources-demo_logs")]
mod demo_logs;
#[cfg(feature = "sources-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sources-exec")]
mod exec;
#[cfg(feature = "sources-haproxy")]
pub mod haproxy;
#[cfg(feature = "sources-internal_logs")]
mod internal_logs;
#[cfg(feature = "sources-internal_metrics")]
mod internal_metrics;
#[cfg(feature = "sources-internal_traces")]
mod internal_traces;
#[cfg(feature = "sources-jaeger")]
mod jaeger;
#[cfg(all(target_os = "linux", feature = "sources-journald"))]
mod journald;
#[cfg(feature = "sources-kafka")]
mod kafka;
#[cfg(feature = "sources-kafka_metrics")]
mod kafka_metrics;
#[cfg(all(target_os = "linux", feature = "sources-kmsg"))]
mod kmsg;
#[cfg(feature = "sources-kubernetes_events")]
mod kubernetes_events;
#[cfg(feature = "sources-kubernetes_logs")]
mod kubernetes_logs;
#[cfg(all(target_os = "linux", feature = "sources-libvirt"))]
mod libvirt;
#[cfg(feature = "sources-memcached")]
mod memcached;
#[cfg(feature = "sources-mongodb")]
mod mongodb;
#[cfg(feature = "sources-mysqld")]
mod mysqld;
#[cfg(feature = "sources-nginx_stub")]
mod nginx_stub;
#[cfg(all(target_os = "linux", feature = "sources-node_metrics"))]
pub mod node;
#[cfg(feature = "sources-ntp")]
mod ntp;
#[cfg(feature = "sources-nvidia_smi")]
mod nvidia_smi;
#[cfg(feature = "sources-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sources-prometheus_scrape")]
mod prometheus_scrape;
#[cfg(feature = "sources-pulsar")]
mod pulsar;
#[cfg(feature = "sources-redis")]
mod redis;
#[cfg(feature = "sources-selfstat")]
mod selfstat;
#[cfg(feature = "sources-syslog")]
pub mod syslog;
#[cfg(feature = "sources-tail")]
mod tail;
#[cfg(feature = "sources-zookeeper")]
mod zookeeper;

use snafu::Snafu;

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("URI parse error: {}", source))]
    UriParseError { source: ::http::uri::InvalidUri },
}
