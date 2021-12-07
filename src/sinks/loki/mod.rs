mod config;
mod sink;
mod healthcheck;
mod event;
mod service;

#[cfg(test)]
mod tests;
mod request_builder;

pub(super) mod proto {
    // include!(concat!(env!("OUT_DIR"), "/loki.rs"));
}

use config::LokiConfig;

inventory::submit! {
    crate::config::SinkDescription::new::<LokiConfig>("loki")
}