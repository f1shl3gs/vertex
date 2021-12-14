mod config;
mod request_builder;
mod service;
mod sink;

#[cfg(test)]
mod tests;

use crate::config::SinkDescription;
use config::KafkaSinkConfig;

inventory::submit! {
    SinkDescription::new::<KafkaSinkConfig>("kafka")
}
