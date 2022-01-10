use std::fmt::Debug;
use std::io::Write;

use async_trait::async_trait;
use buffers::Acker;
use event::encoding::{EncodingConfig, EncodingConfiguration};
use event::Event;
use futures::{stream::BoxStream, FutureExt};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

use crate::{
    config::{DataType, HealthCheck, SinkConfig, SinkContext, SinkDescription},
    impl_generate_config_from_default,
    sinks::{Sink, StreamSink},
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

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Json,
    Text,
}

fn encode_event(mut event: Event, encoding: &EncodingConfig<Encoding>) -> Option<String> {
    encoding.apply_rules(&mut event);

    match event {
        Event::Log(log) => match encoding.codec() {
            Encoding::Json => serde_json::to_string(&log)
                .map_err(|err| {
                    error!(
                        message = "Error encoding json",
                        %err
                    );
                })
                .ok(),

            Encoding::Text => {
                let f = format!("{:?}", log);
                Some(f)
            }
        },
        Event::Metric(metric) => match encoding.codec() {
            Encoding::Json => serde_json::to_string(&metric)
                .map_err(|err| {
                    error!(
                        message = "Error encoding json",
                        %err
                    );
                })
                .ok(),
            Encoding::Text => {
                let f = format!("{:?}", metric);
                Some(f)
            }
        },
    }
}

#[async_trait]
impl StreamSink for StdoutSink {
    async fn run(self: Box<Self>, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        let mut stdout = std::io::stdout();
        let encoding = EncodingConfig::from(Encoding::Json);

        while let Some(mut event) = input.next().await {
            self.acker.ack(1);

            if let Some(text) = encode_event(event, &encoding) {
                stdout.write_all(text.as_bytes()).map_err(|err| {
                    error!(
                        message = "Write event to stdout failed",
                        %err
                    );
                })?;
            }
        }

        Ok(())
    }
}
