mod mock;

use event::Events;
use framework::pipeline::Pipeline;
use futures::Stream;
pub use mock::{MockSinkConfig, MockSourceConfig, MockTransformConfig};

pub fn source() -> (Pipeline, MockSourceConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(1);
    let source = MockSourceConfig::new(rx);
    (tx, source)
}

pub fn transform(suffix: &str, increase: f64) -> MockTransformConfig {
    MockTransformConfig::new(suffix.to_string(), increase)
}

pub fn sink(channel_size: usize) -> (impl Stream<Item = Events>, MockSinkConfig) {
    let (tx, rx) = Pipeline::new_with_buffer(channel_size);
    let sink = MockSinkConfig::new(tx, true);
    (rx, sink)
}
