#[cfg(feature = "extensions-consul_observer")]
mod consul_observer;
#[cfg(feature = "extensions-dns_observer")]
mod dns_observer;
#[cfg(feature = "extensions-exec_observer")]
mod exec_observer;
#[cfg(feature = "extensions-healthcheck")]
pub mod healthcheck;
#[cfg(feature = "extensions-http_observer")]
mod http_observer;
#[cfg(feature = "extensions-kubernetes_observer")]
mod kubernetes_observer;
#[cfg(feature = "extensions-port_observer")]
mod port_observer;
#[cfg(feature = "extensions-pprof")]
mod pprof;
#[cfg(feature = "extensions-zpages")]
pub mod zpages;
