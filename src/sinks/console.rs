use std::fmt::Debug;

use async_trait::async_trait;
use bytes::BytesMut;
use codecs::encoding::{Framer, NewlineDelimitedEncoder, Transformer};
use codecs::{Encoder, EncodingConfig};
use configurable::{configurable_component, Configurable};
use event::{EventContainer, EventStatus, Events, Finalizable};
use framework::{
    config::{DataType, SinkConfig, SinkContext},
    Healthcheck, Sink, StreamSink,
};
use futures::{stream::BoxStream, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tokio_util::codec::Encoder as _;

#[derive(Configurable, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum Stream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum Encoding {
    Json,
    Text,
}

#[configurable_component(sink, name = "console")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The standard stream to write to.
    #[serde(default)]
    stream: Stream,

    encoding: EncodingConfig,
}

#[async_trait]
#[typetag::serde(name = "console")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let transformer = self.encoding.transformer();
        let encoder =
            Encoder::<Framer>::new(NewlineDelimitedEncoder::new().into(), self.encoding.build());

        let sink = match self.stream {
            Stream::Stdout => Sink::Stream(Box::new(WriteSink {
                writer: tokio::io::stdout(),
                transformer,
                encoder,
            })),
            Stream::Stderr => Sink::Stream(Box::new(WriteSink {
                writer: tokio::io::stderr(),
                transformer,
                encoder,
            })),
        };

        Ok((sink, futures::future::ok(()).boxed()))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }
}

struct WriteSink<T> {
    writer: T,
    transformer: Transformer,
    encoder: Encoder<Framer>,
}

#[async_trait]
impl<T> StreamSink for WriteSink<T>
where
    T: tokio::io::AsyncWrite + Send + Sync + Unpin,
{
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        while let Some(events) = input.next().await {
            for mut event in events.into_events() {
                self.transformer.transform(&mut event);

                let finalizers = event.take_finalizers();
                let mut buf = BytesMut::new();
                self.encoder.encode(event, &mut buf).map_err(|_| {
                    // Error is handled by `Encoder`
                    finalizers.update_status(EventStatus::Errored);
                })?;

                match self.writer.write_all(&buf).await {
                    Ok(()) => {
                        finalizers.update_status(EventStatus::Delivered);

                        // TODO: metrics
                    }
                    Err(err) => {
                        error!(
                            message = "Write event to output failed, stopping sink",
                            ?err
                        );

                        finalizers.update_status(EventStatus::Errored);

                        return Err(());
                    }
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
        crate::testing::test_generate_config::<Config>();
    }
}
