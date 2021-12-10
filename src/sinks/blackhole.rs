use async_trait::async_trait;
use buffers::Acker;
use event::Event;
use futures::prelude::stream::BoxStream;
use futures::FutureExt;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::{
    config::{DataType, HealthCheck, SinkConfig, SinkContext, SinkDescription},
    impl_generate_config_from_default,
    sinks::{Sink, StreamSink},
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlackholeConfig {
    pub rate: Option<usize>,
}

inventory::submit! {
    SinkDescription::new::<BlackholeConfig>("blackhole")
}

impl_generate_config_from_default!(BlackholeConfig);

#[async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for BlackholeConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        let sink = BlackholeSink::new(ctx.acker);
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
    pub fn new(acker: Acker) -> Self {
        Self { acker }
    }
}

#[async_trait]
impl StreamSink for BlackholeSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        while let Some(_) = input.next().await {
            self.acker.ack(1);
        }

        Ok(())
    }
}
