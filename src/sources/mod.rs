use futures::future::BoxFuture;

pub type Source = BoxFuture<'static, Result<(), ()>>;

#[cfg(feature = "sources-node_metrics")]
pub mod node;

mod journald;
mod kafka;
mod nginx;
mod internal;
mod zookeeper;
mod prometheus;
mod prometheus_remote_write;
mod pulsar;
mod redis;
mod generator;
mod libvirt;
mod ntp;
mod chrony;
mod blackbox;
mod kubelet;
mod selfstat;
mod kmsg;

