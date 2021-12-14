#![allow(clippy::float_cmp)]

mod async_read;
mod batch;
mod buffers;
mod common;
pub mod config;
mod extensions;
mod http;
mod multiline;
mod partition;
mod pipeline;
mod shutdown;
pub mod signal;
pub mod sinks;
pub mod sources;
mod stats;
mod stream;
mod tcp;
pub mod template;
mod timezone;
mod tls;
pub mod topology;
pub mod trace;
pub mod transforms;
mod trigger;

pub use signal::SignalHandler;

extern crate bloom;

#[macro_use]
extern crate metrics;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate internal;

/// Vertex's basic error type, dynamically dispatched and safe to send across threads
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Vertex's basic result type, defined in terms of [`Error`] and generic over `T`
pub type Result<T> = std::result::Result<T, Error>;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_version() -> String {
    #[cfg(feature = "nightly")]
    let version = format!("{}-nightly", built_info::PKG_VERSION);

    #[cfg(not(feature = "nightly"))]
    let version = format!("{}-stable", built_info::PKG_VERSION);

    version
}

pub fn hostname() -> std::io::Result<String> {
    Ok(::hostname::get()?.to_string_lossy().into())
}
