use std::time::{Duration, Instant};

use async_trait::async_trait;
use configurable::configurable_component;
use event::{EventContainer, Events};
use framework::config::{default_true, DataType, SinkConfig, SinkContext};
use framework::{Healthcheck, Sink, StreamSink};
use futures::stream::BoxStream;
use futures::FutureExt;
use futures_util::StreamExt;
use tokio::time::sleep_until;

#[configurable_component(sink, name = "blackhole")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The number of events, per second, that the sink is allowed to consume.
    ///
    /// By default, there is no limit.
    rate: Option<usize>,

    #[serde(default = "default_true")]
    acknowledgements: bool,
}

#[async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = BlackholeSink::new(self.rate);
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
    }
}

struct BlackholeSink {
    rate: Option<usize>,
    last: Option<Instant>,
}

impl BlackholeSink {
    pub const fn new(rate: Option<usize>) -> Self {
        Self { rate, last: None }
    }
}

#[async_trait]
impl StreamSink for BlackholeSink {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        while let Some(events) = input.next().await {
            if let Some(rate) = self.rate {
                let factor: f32 = 1.0 / rate as f32;
                let secs: f32 = factor * (events.len() as f32);
                let until = self.last.unwrap_or_else(Instant::now) + Duration::from_secs_f32(secs);
                sleep_until(until.into()).await;
                self.last = Some(until);
            }

            // events dropped
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
