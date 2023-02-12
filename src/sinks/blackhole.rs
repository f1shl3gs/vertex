use async_trait::async_trait;
use buffers::Acker;
use configurable::configurable_component;
use event::{EventContainer, Events};
use framework::{
    config::{DataType, SinkConfig, SinkContext},
    Healthcheck, Sink, StreamSink,
};
use futures::prelude::stream::BoxStream;
use futures::FutureExt;
use futures_util::StreamExt;

#[configurable_component(sink, name = "blackhole")]
#[derive(Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct BlackholeConfig {
    /// Receive rate, in event per second.
    pub rate: Option<usize>,
}

#[async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for BlackholeConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = BlackholeSink::new(cx.acker);
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }
}

struct BlackholeSink {
    acker: Acker,
}

impl BlackholeSink {
    pub const fn new(acker: Acker) -> Self {
        Self { acker }
    }
}

#[async_trait]
impl StreamSink for BlackholeSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        while let Some(events) = input.next().await {
            self.acker.ack(events.len());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<BlackholeConfig>()
    }
}
