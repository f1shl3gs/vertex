#[cfg(feature = "extensions-healthcheck")]
pub mod healthcheck;
#[cfg(feature = "extensions-heartbeat")]
pub mod heartbeat;
#[cfg(feature = "extensions-port_observer")]
mod port_observer;
#[cfg(feature = "extensions-pprof")]
mod pprof;
#[cfg(feature = "extensions-zpages")]
pub mod zpages;
