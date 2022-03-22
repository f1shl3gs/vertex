use async_trait::async_trait;
use buffers::Acker;
use event::{EventContainer, Events};
use framework::{
    config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription},
    Healthcheck, Sink, StreamSink,
};
use futures::prelude::stream::BoxStream;
use futures::FutureExt;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlackholeConfig {
    pub rate: Option<usize>,
}

impl GenerateConfig for BlackholeConfig {
    fn generate_config() -> String {
        r#"
# Receive 10 event every second
rate: 10
"#
        .into()
    }
}

inventory::submit! {
    SinkDescription::new::<BlackholeConfig>("blackhole")
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

    fn sink_type(&self) -> &'static str {
        "blackhole"
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
            let n = events.len();

            counter!("blackhole_recv_events_total", n as u64);
            self.acker.ack(n);
        }

        Ok(())
    }
}
