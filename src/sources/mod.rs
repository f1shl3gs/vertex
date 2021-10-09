use futures::future::BoxFuture;

pub type Source = BoxFuture<'static, Result<(), ()>>;

#[cfg(feature = "sources-node_metrics")]
mod node;

#[cfg(feature = "sources-journald")]
mod journald;

#[cfg(feature = "sources-kafka")]
mod kafka;
#[cfg(feature = "sources-nginx_stub")]
mod nginx_stub;
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
#[cfg(feature = "sources-internal_log")]
mod internal_log;