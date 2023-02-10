use std::{path::PathBuf, pin::Pin, time::Duration};

use async_trait::async_trait;
use backoff::ExponentialBackoff;
use bytes::{Bytes, BytesMut};
use codecs::encoding::Transformer;
use configurable::Configurable;
use event::{Event, EventContainer, Events};
use futures::{stream::BoxStream, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{net::UnixStream, time::sleep};
use tokio_util::codec::Encoder;

use crate::batch::EncodedEvent;
use crate::{
    sink::util::{
        socket_bytes_sink::{BytesSink, ShutdownCheck},
        SocketMode,
    },
    sink::VecSinkExt,
};
use crate::{Healthcheck, Sink, StreamSink};

#[derive(Debug, Error)]
pub enum UnixError {
    #[error("Connect error: {0}")]
    Connect(tokio::io::Error),
}

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnixSinkConfig {
    /// The unix socket path. This should be the absolute path.
    #[configurable(required)]
    pub path: PathBuf,
}

impl UnixSinkConfig {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn build(
        &self,
        transformer: Transformer,
        encoder: impl Encoder<Event, Error = codecs::encoding::EncodingError>
            + Clone
            + Send
            + Sync
            + 'static,
    ) -> crate::Result<(Sink, Healthcheck)> {
        let connector = UnixConnector::new(self.path.clone());
        let sink = UnixSink::new(connector.clone(), transformer, encoder);
        Ok((
            Sink::Stream(Box::new(sink)),
            Box::pin(async move { connector.healthcheck().await }),
        ))
    }
}

#[derive(Debug, Clone)]
struct UnixConnector {
    pub path: PathBuf,
}

impl UnixConnector {
    const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    const fn fresh_backoff() -> ExponentialBackoff {
        // TODO: make configurable
        ExponentialBackoff::from_millis(2)
            .factor(250)
            .max_delay(Duration::from_secs(60))
    }

    async fn connect(&self) -> Result<UnixStream, UnixError> {
        UnixStream::connect(&self.path)
            .await
            .map_err(UnixError::Connect)
    }

    async fn connect_backoff(&self) -> UnixStream {
        let mut backoff = Self::fresh_backoff();
        loop {
            match self.connect().await {
                Ok(stream) => {
                    debug!(
                        message = "Connected",
                        ?self.path
                    );
                    // TODO: metrics
                    return stream;
                }
                Err(err) => {
                    error!(
                        message = "Unable to connect",
                        %err,
                        ?self.path
                    );
                    // TODO: metrics
                    sleep(backoff.next().unwrap()).await;
                }
            }
        }
    }

    async fn healthcheck(&self) -> crate::Result<()> {
        self.connect().await.map(|_| ()).map_err(Into::into)
    }
}

struct UnixSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    connector: UnixConnector,
    transformer: Transformer,
    encoder: E,
}

impl<E> UnixSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    pub fn new(connector: UnixConnector, transformer: Transformer, encoder: E) -> Self {
        Self {
            connector,
            transformer,
            encoder,
        }
    }

    async fn connect(&mut self) -> BytesSink<UnixStream> {
        let stream = self.connector.connect_backoff().await;
        BytesSink::new(stream, |_| ShutdownCheck::Alive, SocketMode::Unix)
    }
}

#[async_trait]
impl<E> StreamSink for UnixSink<E>
where
    E: Encoder<Event, Error = codecs::encoding::EncodingError> + Clone + Send + Sync,
{
    // Same as TcpSink, more details there.
    async fn run(mut self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let mut encoder = self.encoder.clone();
        let transformer = self.transformer.clone();

        let mut input = input
            .map(|events| {
                events
                    .into_events()
                    .map(|mut event| {
                        let byte_size = event.size_of();
                        transformer.transform(&mut event);

                        let finalizers = event.metadata_mut().take_finalizers();
                        let mut bytes = BytesMut::new();
                        if encoder.encode(event, &mut bytes).is_ok() {
                            let item = bytes.freeze();

                            EncodedEvent {
                                item,
                                finalizers,
                                byte_size,
                            }
                        } else {
                            EncodedEvent::new(Bytes::new(), 0)
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .flat_map(futures::stream::iter)
            .peekable();

        while Pin::new(&mut input).peek().await.is_some() {
            let mut sink = self.connect().await;

            let result = match sink.send_all_peekable(&mut (&mut input).peekable()).await {
                Ok(()) => sink.close().await,
                Err(err) => Err(err),
            };

            if let Err(err) = result {
                debug!(
                    message = "Unix socket error",
                    %err,
                    path = ?self.connector.path
                );

                // TODO: metrics
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use codecs::encoding::{Framer, NewlineDelimitedEncoder, Serializer, TextSerializer};
    use codecs::Encoder;
    use testify::random::random_lines_with_stream;
    use tokio::net::UnixListener;

    use super::*;
    use crate::testing::CountReceiver;

    fn temp_uds_path(name: &str) -> PathBuf {
        tempfile::tempdir().unwrap().into_path().join(name)
    }

    #[tokio::test]
    async fn unix_sink_healthcheck() {
        let good_path = temp_uds_path("valid_uds");
        let _listener = UnixListener::bind(&good_path).unwrap();
        assert!(UnixSinkConfig::new(good_path)
            .build(
                Default::default(),
                Encoder::<()>::new(Serializer::Text(TextSerializer::new()))
            )
            .unwrap()
            .1
            .await
            .is_ok());

        let bad_path = temp_uds_path("no_one_listening");
        assert!(UnixSinkConfig::new(bad_path)
            .build(
                Default::default(),
                Encoder::<()>::new(Serializer::Text(TextSerializer::new()))
            )
            .unwrap()
            .1
            .await
            .is_err());
    }

    #[tokio::test]
    async fn basic_unix_sink() {
        let num_lines = 1000;
        let out_path = temp_uds_path("unix_test");

        // Set up server to receive events from the Sink.
        let mut receiver = CountReceiver::receive_lines_unix(out_path.clone());

        // Set up Sink
        let config = UnixSinkConfig::new(out_path);

        let (sink, _healthcheck) = config
            .build(
                Default::default(),
                Encoder::<Framer>::new(
                    NewlineDelimitedEncoder::new().into(),
                    TextSerializer::new().into(),
                ),
            )
            .unwrap();

        // Send the test data
        let (input_lines, events) = random_lines_with_stream(100, num_lines, None);
        sink.run(events).await.unwrap();

        // Wait for output to connect
        receiver.connected().await;

        // Receive the data sent by the Sink to the receiver
        assert_eq!(input_lines, receiver.await);
    }
}
