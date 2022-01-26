mod mock;

use crate::util::mock::MockSinkConfig;
use event::Event;
use futures::Stream;
use mock::{MockSourceConfig, MockTransformConfig};
use vertex::pipeline::Pipeline;

pub fn source() -> (Pipeline, MockSourceConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(1);
    let source = MockSourceConfig::new(rx);
    (tx, source)
}

pub fn transform(suffix: &str, increase: f64) -> MockTransformConfig {
    MockTransformConfig::new(suffix.to_string(), increase)
}

pub fn sink(channel_size: usize) -> (impl Stream<Item = Event>, MockSinkConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(channel_size);
    let sink = MockSinkConfig::new(tx, true);
    (rx, sink)
}
