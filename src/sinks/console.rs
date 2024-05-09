use std::fmt::Debug;

use async_trait::async_trait;
use bytes::BytesMut;
use codecs::encoding::{Framer, SinkType, Transformer};
use codecs::{Encoder, EncodingConfigWithFraming};
use configurable::{configurable_component, Configurable};
use event::{EventContainer, EventStatus, Events, Finalizable};
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::{Healthcheck, Sink, StreamSink};
use futures::StreamExt;
use futures::{stream::BoxStream, FutureExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_util::codec::Encoder as _;

/// The [standard stream][standard_streams] to write to.
///
/// [standard_streams]: https://en.wikipedia.org/wiki/Standard_streams
#[derive(Configurable, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
enum Stream {
    /// Write output to `stdout`
    ///
    /// [stdout]: https://en.wikipedia.org/wiki/Standard_streams#Standard_output_(stdout)
    #[default]
    Stdout,

    /// Write output to `stderr`
    ///
    /// [stderr]: https://en.wikipedia.org/wiki/Standard_streams#Standard_error_(stderr)
    Stderr,
}

#[configurable_component(sink, name = "console")]
struct Config {
    /// The standard stream to write to.
    #[serde(default)]
    stream: Stream,

    #[serde(flatten)]
    encoding: EncodingConfigWithFraming,

    #[serde(default)]
    acknowledgements: bool,
}

#[async_trait]
#[typetag::serde(name = "console")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let transformer = self.encoding.transformer();
        let (framer, serializer) = self.encoding.build(SinkType::StreamBased);
        let encoder = Encoder::<Framer>::new(framer, serializer);

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

    fn acknowledgements(&self) -> bool {
        self.acknowledgements
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
        let mut buf = BytesMut::new();

        while let Some(events) = input.next().await {
            for mut event in events.into_events() {
                self.transformer.transform(&mut event);

                let finalizers = event.take_finalizers();
                buf.clear();
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
