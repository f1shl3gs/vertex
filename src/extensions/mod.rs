#[cfg(feature = "extensions-healthcheck")]
pub mod healthcheck;
#[cfg(feature = "extensions-heartbeat")]
pub mod heartbeat;
#[cfg(feature = "extensions-jemalloc")]
mod jemalloc;
#[cfg(feature = "extensions-pprof")]
mod pprof;
#[cfg(feature = "extensions-zpages")]
pub mod zpages;
