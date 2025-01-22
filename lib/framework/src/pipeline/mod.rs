mod errors;

use std::collections::HashMap;

use buffers::channel::{self, LimitedReceiver, LimitedSender};
use bytesize::ByteSizeOf;
use errors::ClosedError;
use event::{EventContainer, Events};
use futures::Stream;
use futures_util::StreamExt;
use metrics::{Attributes, Counter};

use crate::config::Output;

const CHUNK_SIZE: usize = 1024;
pub const DEFAULT_OUTPUT: &str = "_default";

#[derive(Debug)]
pub struct Builder {
    buf_size: usize,
    inner: Option<Inner>,
    named_inners: HashMap<String, Inner>,
}

impl Builder {
    pub fn with_buffer(self, buf_size: usize) -> Self {
        Self {
            buf_size,
            inner: self.inner,
            named_inners: self.named_inners,
        }
    }

    pub fn add_output(
        &mut self,
        component: impl Into<String>,
        component_type: &'static str,
        output: Output,
    ) -> LimitedReceiver<Events> {
        match output.port {
            None => {
                let (inner, rx) = Inner::new_with_buffer(
                    self.buf_size,
                    component.into(),
                    component_type.into(),
                    DEFAULT_OUTPUT.to_owned(),
                );
                self.inner = Some(inner);

                rx
            }
            Some(name) => {
                let (inner, rx) = Inner::new_with_buffer(
                    self.buf_size,
                    component.into(),
                    component_type.into(),
                    name.to_owned(),
                );
                self.named_inners.insert(name, inner);

                rx
            }
        }
    }

    pub fn build(self) -> Pipeline {
        Pipeline {
            inner: self.inner,
            named_inners: self.named_inners,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pipeline {
    inner: Option<Inner>,
    named_inners: HashMap<String, Inner>,
}

impl Pipeline {
    pub fn builder() -> Builder {
        Builder {
            buf_size: CHUNK_SIZE,
            inner: None,
            named_inners: Default::default(),
        }
    }

    pub async fn send(&mut self, events: impl Into<Events>) -> Result<(), ClosedError> {
        self.inner
            .as_mut()
            .expect("no default output")
            .send(events.into())
            .await
    }

    pub async fn send_named(&mut self, name: &str, events: Events) -> Result<(), ClosedError> {
        self.named_inners
            .get_mut(name)
            .expect("unknown output")
            .send(events)
            .await
    }

    pub async fn send_batch<E, I>(&mut self, events: I) -> Result<(), ClosedError>
    where
        E: Into<Events> + ByteSizeOf,
        I: IntoIterator<Item = E>,
    {
        self.inner
            .as_mut()
            .expect("no default output")
            .send_batch(events)
            .await
    }

    pub async fn send_stream<S, E>(&mut self, stream: S) -> Result<(), ClosedError>
    where
        S: Stream<Item = E> + Unpin,
        E: Into<Events> + ByteSizeOf,
    {
        self.inner
            .as_mut()
            .expect("no default output")
            .send_stream(stream)
            .await
    }
}

#[cfg(any(test, feature = "test-util"))]
impl Pipeline {
    pub fn new_with_buffer(n: usize) -> (Self, LimitedReceiver<Events>) {
        let (inner, rx) =
            Inner::new_with_buffer(n, "".into(), "".into(), DEFAULT_OUTPUT.to_owned());

        (
            Self {
                inner: Some(inner),
                named_inners: Default::default(),
            },
            rx,
        )
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test() -> (Self, impl Stream<Item = Events> + Unpin) {
        let (pipe, recv) = Self::new_with_buffer(100);

        (pipe, recv.into_stream())
    }

    #[cfg(feature = "test-util")]
    pub fn new_test_finalize(
        status: event::EventStatus,
    ) -> (Self, impl Stream<Item = Events> + Unpin) {
        use event::Finalizable;

        let (pipe, recv) = Self::new_with_buffer(100);

        // In a source test pipeline, there is no sink to acknowledge events,
        // so we have to add a map to the receiver to handle the finalization
        let recv = recv.into_stream().map(move |mut events| {
            let mut finalizers = events.take_finalizers();
            finalizers.update_status(status);
            finalizers.update_sources();

            events
        });

        (pipe, recv)
    }

    #[cfg(test)]
    pub fn add_outputs(
        &mut self,
        status: event::EventStatus,
        name: String,
    ) -> impl Stream<Item = Events> + Unpin {
        let (inner, recv) = Inner::new_with_buffer(100, "".into(), "".into(), name.clone());
        let recv = recv.into_stream().map(move |mut events| {
            events.for_each_event(|mut event| {
                let metadata = event.metadata_mut();
                metadata.update_status(status);
                metadata.update_sources();
            });

            events
        });

        self.named_inners.insert(name, inner);
        recv
    }
}

#[derive(Clone, Debug)]
struct Inner {
    inner: LimitedSender<Events>,

    // metrics
    sent_events: Counter,
    sent_bytes: Counter,
}

impl Inner {
    fn new_with_buffer(
        n: usize,
        component: String,
        component_type: String,
        output: String,
    ) -> (Self, LimitedReceiver<Events>) {
        let (tx, rx) = channel::limited(n);

        let attrs = Attributes::from([
            ("output", output.into()),
            ("component", component.into()),
            ("component_kind", "source".into()),
            ("component_type", component_type.into()),
        ]);
        let sent_events = metrics::register_counter(
            "component_sent_events_total",
            "The total number of events emitted by this component.",
        )
        .recorder(attrs.clone());
        let sent_bytes = metrics::register_counter(
            "component_sent_event_bytes_total",
            "The total number of event bytes emitted by this component.",
        )
        .recorder(attrs);

        (
            Self {
                inner: tx,
                sent_events,
                sent_bytes,
            },
            rx,
        )
    }

    async fn send(&mut self, events: Events) -> Result<(), ClosedError> {
        let count = events.len();
        let byte_size = events.size_of();

        self.inner.send(events).await.map_err(|_err| ClosedError)?;

        self.sent_events.inc(count as u64);
        self.sent_bytes.inc(byte_size as u64);

        Ok(())
    }

    async fn send_batch<E, B>(&mut self, batch: B) -> Result<(), ClosedError>
    where
        E: Into<Events> + ByteSizeOf,
        B: IntoIterator<Item = E>,
    {
        let mut count = 0;
        let mut byte_size = 0;

        for events in batch.into_iter().map(Into::into) {
            let n = events.len();
            let s = events.size_of();

            match self.inner.send(events).await {
                Ok(()) => {
                    count += n;
                    byte_size += s;
                }

                Err(_err) => {
                    self.sent_events.inc(count as u64);
                    self.sent_bytes.inc(byte_size as u64);

                    trace!(
                        message = "Events send failed",
                        %count,
                        %byte_size
                    );

                    return Err(ClosedError);
                }
            }
        }

        self.sent_events.inc(count as u64);
        self.sent_bytes.inc(byte_size as u64);

        trace!(
            message = "Events send failed",
            %count,
            %byte_size
        );

        Ok(())
    }

    async fn send_stream<S, E>(&mut self, mut stream: S) -> Result<(), ClosedError>
    where
        S: Stream<Item = E> + Unpin,
        E: Into<Events> + ByteSizeOf,
    {
        while let Some(events) = stream.next().await {
            self.send(events.into()).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, TimeDelta, Utc};
    use event::LogRecord;
    use rand::{thread_rng, Rng};

    #[tokio::test]
    async fn emits_lag_time_for_log() {
        emit_and_test(|timestamp| {
            let mut log = LogRecord::from("log message");
            log.insert(log_schema::log_schema().timestamp_key(), timestamp);
            log.into()
        })
        .await
    }

    async fn emit_and_test(make_event: impl FnOnce(DateTime<Utc>) -> Events) {
        let (mut sender, _stream) = Pipeline::new_test();
        let millis = thread_rng().gen_range(10..10000);
        let timestamp = Utc::now() - TimeDelta::try_milliseconds(millis).unwrap();
        let _expected = millis as f64 / 1000.0;

        let event = make_event(timestamp);
        sender.send(event).await.expect("Send should not fail");
    }
}
