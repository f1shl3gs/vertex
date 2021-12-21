#[cfg(feature = "sources-bind")]
mod bind;
mod blackbox;
mod chrony;
#[cfg(feature = "sources-consul")]
mod consul;
#[cfg(feature = "sources-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sources-fluentd")]
mod fluentd;
#[cfg(feature = "sources-generator")]
mod generator;
#[cfg(feature = "sources-haproxy")]
pub mod haproxy;
#[cfg(feature = "sources-internal_logs")]
mod internal_logs;
#[cfg(feature = "sources-internal_metrics")]
mod internal_metrics;
#[cfg(all(target_os = "linux", feature = "sources-journald"))]
mod journald;
#[cfg(feature = "sources-kafka")]
mod kafka;
#[cfg(feature = "sources-kafka_metrics")]
mod kafka_metrics;
#[cfg(feature = "sources-kmsg")]
mod kmsg;
mod kube_events;
mod kube_state_metrics;
mod kubelet;
#[cfg(feature = "sources-libvirt")]
mod libvirt;
#[cfg(feature = "sources-memcached")]
mod memcached;
#[cfg(feature = "sources-mongodb")]
mod mongodb;
#[cfg(feature = "sources-mysqld")]
mod mysqld;
#[cfg(feature = "sources-nginx_stub")]
mod nginx_stub;
#[cfg(all(unix, feature = "sources-node_metrics"))]
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
mod syslog;
#[cfg(feature = "sources-tail")]
mod tail;
mod utils;
#[cfg(feature = "sources-zookeeper")]
mod zookeeper;

use futures::future::BoxFuture;
use snafu::Snafu;

pub type Source = BoxFuture<'static, Result<(), ()>>;

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("URI parse error: {}", source))]
    UriParseError { source: ::http::uri::InvalidUri },
}
