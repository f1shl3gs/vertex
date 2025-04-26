#![allow(dead_code)]

mod mock;
mod topology;
mod trace;

use buffer::LimitedReceiver;
use event::Events;
use framework::pipeline::Pipeline;
pub use mock::{MockSinkConfig, MockSourceConfig, MockTransformConfig};
pub use topology::start_topology;
pub use trace::trace_init;

pub fn source() -> (Pipeline, MockSourceConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(128 * 1024);
    let source = MockSourceConfig::new(rx);
    (tx, source)
}

pub fn source_with_buffer(buffer: usize) -> (Pipeline, MockSourceConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(buffer);
    let source = MockSourceConfig::new(rx);
    (tx, source)
}

pub fn source_with_data(data: &str) -> (Pipeline, MockSourceConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(128 * 1024);
    let source = MockSourceConfig::new_with_data(rx, data);
    (tx, source)
}

pub fn transform(suffix: &str, increase: f64) -> MockTransformConfig {
    MockTransformConfig::new(suffix.to_string(), increase)
}

pub fn sink() -> (LimitedReceiver<Events>, MockSinkConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(128 * 1024);
    let sink = MockSinkConfig::new(tx, true);
    (rx, sink)
}

pub fn sink_with_data(data: &str) -> (LimitedReceiver<Events>, MockSinkConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(128 * 1024);
    let sink = MockSinkConfig::new_with_data(tx, true, data);
    (rx, sink)
}

pub fn sink_failing_healthcheck(channel_size: usize) -> (LimitedReceiver<Events>, MockSinkConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(channel_size);
    let sink = MockSinkConfig::new(tx, false);
    (rx, sink)
}
