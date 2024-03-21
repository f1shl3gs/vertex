mod config;
mod request_builder;
mod sanitize;
mod service;
mod sink;

pub use config::valid_label_name;

#[cfg(all(test, feature = "integration-tests-loki"))]
mod integration_tests;
#[cfg(test)]
mod tests;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loki.rs"));
}
