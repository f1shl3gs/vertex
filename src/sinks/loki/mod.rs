mod config;
mod event;
mod healthcheck;
mod request_builder;
mod service;
mod sink;

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
