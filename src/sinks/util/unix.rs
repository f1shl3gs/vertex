use std::{path::PathBuf, pin::Pin, sync::Arc, time::Duration};

use async_trait::async_trait;
use buffers::Acker;
use bytes::Bytes;
use event::Event;
use futures::{stream::BoxStream, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tokio::{net::UnixStream, time::sleep};

use crate::batch::EncodedEvent;
use crate::sinks::util::SocketMode;
use crate::sinks::StreamSink;
use crate::{
    config::SinkContext,
    sink::VecSinkExt,
    sinks::{
        util::{
            retries::ExponentialBackoff,
            socket_bytes_sink::{BytesSink, ShutdownCheck},
        },
        Healthcheck, Sink,
    },
};

#[derive(Debug, Snafu)]
pub enum UnixError {
    #[snafu(display("Connect error: {}", source))]
    ConnectError { source: tokio::io::Error },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnixSinkConfig {
    pub path: PathBuf,
}

impl UnixSinkConfig {
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn build(
        &self,
        cx: SinkContext,
        encode_event: impl Fn(Event) -> Option<Bytes> + Send + Sync + 'static,
    ) -> crate::Result<(Sink, Healthcheck)> {
        let connector = UnixConnector::new(self.path.clone());
        let sink = UnixSink::new(connector.clone(), cx.acker(), encode_event);
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
        UnixStream::connect(&self.path).await.context(ConnectError)
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

struct UnixSink {
    connector: UnixConnector,
    acker: Acker,
    encode_event: Arc<dyn Fn(Event) -> Option<Bytes> + Send + Sync>,
}

impl UnixSink {
    pub fn new(
        connector: UnixConnector,
        acker: Acker,
        encode_event: impl Fn(Event) -> Option<Bytes> + Send + Sync + 'static,
    ) -> Self {
        Self {
            connector,
            acker,
            encode_event: Arc::new(encode_event),
        }
    }

    async fn connect(&mut self) -> BytesSink<UnixStream> {
        let stream = self.connector.connect_backoff().await;
        BytesSink::new(
            stream,
            |_| ShutdownCheck::Alive,
            self.acker.clone(),
            SocketMode::Unix,
        )
    }
}

#[async_trait]
impl StreamSink for UnixSink {
    // Same as TcpSink, more details there.
    async fn run(mut self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        let encode_event = Arc::clone(&self.encode_event);
        let mut input = input
            .map(|mut event| {
                let byte_size = event.size_of();
                let finalizers = event.metadata_mut().take_finalizers();
                encode_event(event)
                    .map(|item| EncodedEvent {
                        item,
                        finalizers,
                        byte_size,
                    })
                    .unwrap_or_else(|| EncodedEvent::new(Bytes::new(), 0))
            })
            .peekable();

        while Pin::new(&mut input).peek().await.is_some() {
            let mut sink = self.connect().await;

            let result = match sink
                .send_all_peekable(&mut (&mut input).map(|item| item.item).peekable())
                .await
            {
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
    use testify::random::random_lines_with_stream;
    use tokio::net::UnixListener;

    use super::*;
    use crate::sinks::util::{encode_log, Encoding};
    use crate::testing::CountReceiver;

    fn temp_uds_path(name: &str) -> PathBuf {
        tempfile::tempdir().unwrap().into_path().join(name)
    }

    #[tokio::test]
    async fn unix_sink_healthcheck() {
        let good_path = temp_uds_path("valid_uds");
        let _listener = UnixListener::bind(&good_path).unwrap();
        assert!(UnixSinkConfig::new(good_path)
            .build(SinkContext::new_test(), |_| None)
            .unwrap()
            .1
            .await
            .is_ok());

        let bad_path = temp_uds_path("no_one_listening");
        assert!(UnixSinkConfig::new(bad_path)
            .build(SinkContext::new_test(), |_| None)
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
        let cx = SinkContext::new_test();
        let encoding = Encoding::Text.into();
        let (sink, _healthcheck) = config
            .build(cx, move |event| encode_log(event, &encoding))
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
