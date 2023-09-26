use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use framework::{
    config::{DataType, SinkConfig, SinkContext},
    Healthcheck, Sink, StreamSink,
};
use futures::prelude::stream::BoxStream;
use futures::FutureExt;
use futures_util::StreamExt;

#[configurable_component(sink, name = "blackhole")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Receive rate, in event per second.
    pub rate: Option<usize>,
}

#[async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = BlackholeSink::new();
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }
}

struct BlackholeSink {}

impl BlackholeSink {
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl StreamSink for BlackholeSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        while let Some(_events) = input.next().await {
            // do something !?
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
