#![allow(clippy::new_without_default)]
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]
#![deny(clippy::clone_on_ref_ptr)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![deny(clippy::disallowed_method)] // [nursery] mark some functions as verboten
#![deny(clippy::missing_const_for_fn)] // [nursery] valuable to the optimizer, but may produce false positives

mod async_read;
mod batch;
mod codecs;
mod common;
pub mod config;
mod dns;
mod encoding_transcode;
pub mod extensions;
mod http;
mod multiline;
mod partition;
pub mod pipeline;
mod shutdown;
pub mod signal;
mod sink;
pub mod sinks;
pub mod sources;
mod stats;
mod stream;
pub mod tcp;
pub mod template;
mod tls;
pub mod topology;
pub mod trace;
pub mod transforms;
mod trigger;
pub mod udp;
pub mod utilization;

pub mod testing;

pub use signal::SignalHandler;

#[macro_use]
extern crate metrics;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate internal;
extern crate core;

/// Vertex's basic error type, dynamically dispatched and safe to send across threads
pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Vertex's basic result type, defined in terms of [`Error`] and generic over `T`
pub type Result<T> = std::result::Result<T, Error>;

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_version() -> String {
    let pkg_version = built_info::PKG_VERSION.to_string();
    let build_desc = built_info::VERTEX_BUILD_DESC;
    let build_string = match build_desc {
        Some(desc) => format!("{} {}", built_info::TARGET, desc),
        None => built_info::TARGET.to_string(),
    };

    // We do not add 'debug' to the BUILD_DESC unless the caller has
    // flagged on line or full debug symbols. See the Cargo Book profiling
    // section for value meaning:
    // https://doc.rust-lang.org/cargo/reference/profiles.html#debug
    let build_string = match built_info::DEBUG {
        "1" => format!("{} debug=line", build_string),
        "2" | "true" => format!("{} debug=full", build_string),
        _ => build_string,
    };

    format!("{} ({})", pkg_version, build_string)
}

pub fn hostname() -> std::io::Result<String> {
    Ok(::hostname::get()?.to_string_lossy().into())
}
