mod config;
mod request_builder;
mod service;
mod sink;

#[cfg(test)]
mod tests;

use config::KafkaSinkConfig;
use framework::config::SinkDescription;

inventory::submit! {
    SinkDescription::new::<KafkaSinkConfig>("kafka")
}
