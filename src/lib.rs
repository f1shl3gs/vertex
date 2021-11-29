pub mod config;
pub mod transforms;
pub mod sources;
pub mod topology;
pub mod trace;
pub mod signal;
mod shutdown;
mod sinks;
mod timezone;
mod pipeline;
mod buffers;
mod tls;
mod trigger;
mod extensions;
mod http;
mod template;
mod multiline;
mod common;
mod batch;
mod tcp;
mod async_read;
mod stream;
mod partition;

pub use signal::{SignalHandler};

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