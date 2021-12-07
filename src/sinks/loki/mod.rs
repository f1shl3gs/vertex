mod config;
mod sink;
mod healthcheck;
mod event;
mod service;
mod request_builder;

pub use config::valid_label_name;

#[cfg(test)]
mod tests;

pub(super) mod proto {
    include!(concat!(env!("OUT_DIR"), "/loki.rs"));
}

use config::LokiConfig;

inventory::submit! {
    crate::config::SinkDescription::new::<LokiConfig>("loki")
}