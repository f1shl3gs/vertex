use std::fmt::Debug;

use async_trait::async_trait;
use buffers::Acker;
use event::{Event, EventContainer, Events};
use framework::sink::util::encoding::{EncodingConfig, EncodingConfiguration};
use framework::{
    config::{DataType, GenerateConfig, SinkConfig, SinkContext, SinkDescription},
    Healthcheck, Sink, StreamSink,
};
use futures::{stream::BoxStream, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Stream {
    Stdout,
    Stderr,
}

impl Default for Stream {
    fn default() -> Self {
        Self::Stdout
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Json,
    Text,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConsoleSinkConfig {
    #[serde(default)]
    stream: Stream,

    encoding: EncodingConfig<Encoding>,
}

impl GenerateConfig for ConsoleSinkConfig {
    fn generate_config() -> String {
        r#"
stream: stdout
encoding: text
"#
        .into()
    }
}

inventory::submit! {
    SinkDescription::new::<ConsoleSinkConfig>("console")
}

#[async_trait]
#[typetag::serde(name = "console")]
impl SinkConfig for ConsoleSinkConfig {
    async fn build(&self, cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = match self.stream {
            Stream::Stdout => Sink::Stream(Box::new(WriteSink {
                acker: cx.acker,
                writer: tokio::io::stdout(),
                encoding: self.encoding.clone(),
            })),
            Stream::Stderr => Sink::Stream(Box::new(WriteSink {
                acker: cx.acker,
                writer: tokio::io::stderr(),
                encoding: self.encoding.clone(),
            })),
        };

        Ok((sink, futures::future::ok(()).boxed()))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn sink_type(&self) -> &'static str {
        "console"
    }
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
        Event::Trace(trace) => match encoding.codec() {
            Encoding::Json => serde_json::to_string(&trace)
                .map_err(|err| {
                    error!(
                        message = "Error encoding json",
                        %err
                    );
                })
                .ok(),

            Encoding::Text => Some(format!("{:?}", trace)),
        },
    }
}

struct WriteSink<T> {
    acker: Acker,
    writer: T,
    encoding: EncodingConfig<Encoding>,
}

#[async_trait]
impl<T> StreamSink for WriteSink<T>
where
    T: tokio::io::AsyncWrite + Send + Sync + Unpin,
{
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        let encoding = self.encoding;

        while let Some(events) = input.next().await {
            self.acker.ack(events.len());

            for event in events.into_events() {
                if let Some(mut text) = encode_event(event, &encoding) {
                    // Without the new line char, the latest line will be buffered
                    // rather than flush to terminal immediately.
                    text.push('\n');

                    self.writer
                        .write_all(text.as_bytes())
                        .await
                        .map_err(|err| {
                            error!(
                                message = "Write event to output failed",
                                %err
                            );
                        })?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<ConsoleSinkConfig>();
    }
}
