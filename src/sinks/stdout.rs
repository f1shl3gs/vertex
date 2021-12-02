use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use event::Event;
use tokio_stream::StreamExt;
use futures::{
    FutureExt,
    stream::{ BoxStream }
};

use crate::{
    buffers::Acker,
    impl_generate_config_from_default,
    config::{SinkConfig, SinkContext, DataType, HealthCheck, SinkDescription},
    sinks::{Sink, StreamSink}
};


#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StdoutConfig {}

inventory::submit! {
    SinkDescription::new::<StdoutConfig>("stdout")
}

impl_generate_config_from_default!(StdoutConfig);

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
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        while let Some(event) = input.next().await {
            self.acker.ack(1);

            match event {
                Event::Metric(m) => {
                    println!("{:?}", m)
                },

                Event::Log(l) => {
                    println!("{:?}", l)
                }
            }
        }

        Ok(())
    }
}