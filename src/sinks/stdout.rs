use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::sinks::{Sink, StreamSink};
use crate::config::{SinkConfig, SinkContext, DataType, HealthCheck};
use crate::buffers::Acker;
use futures::{
    FutureExt,
    stream::{ BoxStream }
};
use crate::event::Event;
use tokio_stream::StreamExt;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct StdoutConfig {}

#[async_trait]
#[typetag::serde(name = "stdout")]
impl SinkConfig for StdoutConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        Ok((
            Sink::Stream(Box::new(StdoutSink { acker: ctx.acker })),
            futures::future::ok(()).boxed(),
        ))
    }


    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn sink_type(&self) -> &'static str {
        "stdout"
    }
}

struct StdoutSink {
    acker: Acker,
}

#[async_trait]
impl StreamSink for StdoutSink {
    async fn run(&mut self, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        while let Some(event) = input.next().await {
            self.acker.ack(1);
            let metric = event.as_metric();
            println!("STDOUT {:?}", metric);
        }

        Ok(())
    }
}