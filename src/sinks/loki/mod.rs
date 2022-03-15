mod config;
mod event;
mod healthcheck;
mod request_builder;
mod service;
mod sink;

pub use config::valid_label_name;

#[cfg(all(test, feature = "integration-tests-loki"))]
mod integration_tests;
mod sanitize;
#[cfg(test)]
mod tests;

pub(super) mod proto {
    include!(concat!(env!("OUT_DIR"), "/loki.rs"));
}

use config::LokiConfig;

inventory::submit! {
    framework::config::SinkDescription::new::<LokiConfig>("loki")
}
