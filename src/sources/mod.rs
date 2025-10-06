#[cfg(feature = "sources-audit")]
mod audit;
#[cfg(feature = "sources-bind")]
mod bind;
#[cfg(feature = "sources-chrony")]
mod chrony;
#[cfg(feature = "sources-clickhouse_metrics")]
mod clickhouse_metrics;
#[cfg(feature = "sources-consul")]
mod consul;
#[cfg(feature = "sources-dnsmasq")]
mod dnsmasq;
#[cfg(feature = "sources-dnstap")]
mod dnstap;
#[cfg(feature = "sources-docker")]
mod docker;
#[cfg(feature = "sources-dpdk")]
mod dpdk;
#[cfg(feature = "sources-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sources-exec")]
mod exec;
#[cfg(feature = "sources-filestats")]
mod filestats;
#[cfg(feature = "sources-fluent")]
mod fluent;
#[cfg(feature = "sources-generate")]
mod generate;
#[cfg(feature = "sources-grpc_check")]
mod grpc_check;
#[cfg(feature = "sources-haproxy")]
pub mod haproxy;
#[cfg(feature = "sources-http")]
mod http;
#[cfg(feature = "sources-http_check")]
mod http_check;
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
#[cfg(feature = "sources-mqtt")]
mod mqtt;
#[cfg(feature = "sources-multiplier")]
mod multiplier;
#[cfg(feature = "sources-mysqld")]
mod mysqld;
#[cfg(feature = "sources-netflow")]
mod netflow;
#[cfg(feature = "sources-nginx_stub")]
mod nginx_stub;
#[cfg(all(target_os = "linux", feature = "sources-node"))]
pub mod node;
#[cfg(feature = "sources-ntp")]
mod ntp;
#[cfg(feature = "sources-nvidia")]
mod nvidia;
#[cfg(feature = "sources-ping")]
mod ping;
#[cfg(feature = "sources-podman")]
mod podman;
#[cfg(feature = "sources-prometheus_pushgateway")]
mod prometheus_pushgateway;
#[cfg(feature = "sources-prometheus_remote_write")]
mod prometheus_remote_write;
#[cfg(feature = "sources-prometheus_scrape")]
mod prometheus_scrape;
#[cfg(feature = "sources-prometheus_textfile")]
mod prometheus_textfile;
#[cfg(feature = "sources-pulsar")]
mod pulsar;
#[cfg(feature = "sources-redfish")]
mod redfish;
#[cfg(feature = "sources-redis")]
mod redis;
#[cfg(feature = "sources-selfstat")]
mod selfstat;
#[cfg(feature = "sources-sflow")]
mod sflow;
#[cfg(feature = "sources-socket")]
pub mod socket;
#[cfg(feature = "sources-static_metrics")]
mod static_metrics;
#[cfg(feature = "sources-syslog")]
pub mod syslog;
#[cfg(feature = "sources-systemd")]
mod systemd;
#[cfg(feature = "sources-tail")]
mod tail;
#[cfg(feature = "sources-zookeeper")]
mod zookeeper;

use codecs::decoding::{DeserializerConfig, FramingConfig};

pub const fn default_decoding() -> DeserializerConfig {
    DeserializerConfig::Bytes
}

pub const fn default_framing_message_based() -> FramingConfig {
    FramingConfig::Bytes
}
