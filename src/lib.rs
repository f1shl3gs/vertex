#![deny(unused_qualifications)]
#![allow(clippy::new_without_default)]
#![allow(clippy::float_cmp)]
#![allow(clippy::too_many_arguments)]
#![deny(clippy::clone_on_ref_ptr)]
#![deny(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::type_complexity)]
#![deny(clippy::disallowed_methods)] // [nursery] mark some functions as verboten

pub mod common;
pub mod extensions;
pub mod sinks;
pub mod sources;
pub mod transforms;

#[cfg(test)]
pub mod testing;

#[macro_use]
extern crate tracing;

pub use framework::{Error, Result};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

pub fn get_version() -> String {
    let pkg_version = built_info::PKG_VERSION.to_string();

    // We do not add 'debug' to the BUILD_DESC unless the caller has
    // flagged on line or full debug symbols. See the Cargo Book profiling
    // section for value meaning:
    // https://doc.rust-lang.org/cargo/reference/profiles.html#debug
    let build_string = match built_info::DEBUG {
        "1" => format!("{} debug=line", built_info::TARGET),
        "2" | "true" => format!("{} debug=full", built_info::TARGET),
        _ => built_info::TARGET.to_string(),
    };

    format!("{} ({})", pkg_version, build_string)
}
