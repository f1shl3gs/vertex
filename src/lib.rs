#![allow(clippy::new_without_default)]
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]
#![deny(clippy::clone_on_ref_ptr)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::type_complexity)]
#![deny(clippy::disallowed_methods)] // [nursery] mark some functions as verboten
#![deny(clippy::missing_const_for_fn)] // [nursery] valuable to the optimizer, but may produce false positives

pub mod common;
pub mod extensions;
pub mod sinks;
pub mod sources;
pub mod transforms;

#[cfg(test)]
pub mod testing;

pub use framework::hostname;

#[macro_use]
extern crate tracing;

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
