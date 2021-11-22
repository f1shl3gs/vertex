#[cfg(feature = "sources-node_metrics")]
pub mod node;
#[cfg(feature = "sources-journald")]
mod journald;
#[cfg(feature = "sources-kafka")]
mod kafka;
#[cfg(feature = "sources-nginx_stub")]
mod nginx_stub;
#[cfg(feature = "sources-zookeeper")]
mod zookeeper;
#[cfg(feature = "sources-prometheus_scrape")]
mod prometheus_scrape;
#[cfg(feature = "sources-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sources-pulsar")]
mod pulsar;
#[cfg(feature = "sources-redis")]
mod redis;
#[cfg(feature = "sources-generator")]
mod generator;
#[cfg(feature = "sources-libvirt")]
mod libvirt;
#[cfg(feature = "sources-ntp")]
mod ntp;
mod chrony;
mod blackbox;
mod kubelet;
#[cfg(feature = "sources-selfstat")]
mod selfstat;
#[cfg(feature = "sources-kmsg")]
mod kmsg;
#[cfg(feature = "sources-internal_metrics")]
mod internal_metrics;
#[cfg(feature = "sources-internal_logs")]
mod internal_logs;
#[cfg(feature = "sources-bind")]
mod bind;
#[cfg(feature = "sources-haproxy")]
mod haproxy;
#[cfg(feature = "sources-memcached")]
mod memcached;
#[cfg(feature = "sources-fluentd")]
mod fluentd;
#[cfg(feature = "sources-syslog")]
mod syslog;
#[cfg(feature = "sources-tail")]
mod tail;
#[cfg(feature = "sources-kafka_metrics")]
mod kafka_metrics;
#[cfg(feature = "sources-mysqld")]
mod mysqld;
mod kube_events;
mod kube_state_metrics;
#[cfg(feature = "sources-nvidia_smi")]
mod nvidia_smi;
#[cfg(feature = "sources-mongodb")]
mod mongodb;

use snafu::Snafu;
use futures::future::BoxFuture;

pub type Source = BoxFuture<'static, Result<(), ()>>;

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("URI parse error: {}", source))]
    UriParseError { source: ::http::uri::InvalidUri }
}