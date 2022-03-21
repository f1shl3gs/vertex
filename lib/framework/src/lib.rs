#![allow(clippy::type_complexity)]

pub mod async_read;
pub mod batch;
pub mod codecs;
mod common;
pub mod config;
pub mod dns;
mod extension;
pub mod http;
pub mod partition;
pub mod pipeline;
pub mod shutdown;
pub mod signal;
pub mod sink;
pub mod source;
pub mod stats;
pub mod stream;
pub mod tcp;
pub mod template;
#[cfg(any(test, feature = "test-util"))]
pub mod testing;
pub mod timezone;
pub mod tls;
pub mod topology;
pub mod trace;
mod transform;
pub mod trigger;
pub mod udp;
mod utilization;

pub use common::*;
pub use extension::Extension;
pub use pipeline::Pipeline;
pub use shutdown::*;
pub use signal::*;
pub use sink::{Healthcheck, HealthcheckError, Sink, StreamSink};
pub use source::Source;
pub use transform::{FunctionTransform, OutputBuffer, SyncTransform, TaskTransform, Transform};
pub(crate) use transform::{TransformOutputs, TransformOutputsBuf};

#[macro_use]
extern crate internal;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate metrics;
#[macro_use]
extern crate tracing;

/// Vertex's basic error type, dynamically dispatched and safe to send across threads
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Vertex's basic result type, defined in terms of [`Error`] and generic over `T`
pub type Result<T> = std::result::Result<T, Error>;

pub fn hostname() -> std::io::Result<String> {
    Ok(::hostname::get()?.to_string_lossy().into())
}

pub fn get_version() -> String {
    // TODO
    "0.1.0".into()
}
