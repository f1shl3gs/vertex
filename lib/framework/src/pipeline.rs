use std::collections::HashMap;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use event::Event;
use futures::Stream;
use futures_util::StreamExt;
use internal::EventsSent;
use pin_project::pin_project;
use shared::ByteSizeOf;
use tokio::sync::mpsc;

use crate::config::Output;

const CHUNK_SIZE: usize = 1000;
pub const DEFAULT_OUTPUT: &str = "_default";

#[derive(Debug)]
pub struct ClosedError;

impl fmt::Display for ClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Sender is closed.")
    }
}

impl std::error::Error for ClosedError {}

impl From<mpsc::error::SendError<Event>> for ClosedError {
    fn from(_: mpsc::error::SendError<Event>) -> Self {
        Self
    }
}

#[derive(Debug)]
pub enum StreamSendError<E> {
    Closed(ClosedError),
    Stream(E),
}

impl<E> fmt::Display for StreamSendError<E>
where
    E: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamSendError::Closed(e) => e.fmt(f),
            StreamSendError::Stream(e) => e.fmt(f),
        }
    }
}

impl<E> std::error::Error for StreamSendError<E> where E: std::error::Error {}

impl<E> From<ClosedError> for StreamSendError<E> {
    fn from(e: ClosedError) -> Self {
        StreamSendError::Closed(e)
    }
}

#[derive(Debug)]
pub struct Builder {
    buf_size: usize,
    inner: Option<Inner>,
    named_inners: HashMap<String, Inner>,
}

impl Builder {
    // https://github.com/rust-lang/rust/issues/73255
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_buffer(self, buf_size: usize) -> Self {
        Self {
            buf_size,
            inner: self.inner,
            named_inners: self.named_inners,
        }
    }

    pub fn add_output(&mut self, output: Output) -> ReceiverStream<Event> {
        match output.port {
            None => {
                let (inner, rx) = Inner::new_with_buffer(self.buf_size, DEFAULT_OUTPUT.to_owned());
                self.inner = Some(inner);

                rx
            }
            Some(name) => {
                let (inner, rx) = Inner::new_with_buffer(self.buf_size, name.to_owned());
                self.named_inners.insert(name, inner);

                rx
            }
        }
    }

    // https://github.com/rust-lang/rust/issues/73255
    #[allow(clippy::missing_const_for_fn)]
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

    pub fn new_with_buffer(n: usize) -> (Self, ReceiverStream<Event>) {
        let (inner, rx) = Inner::new_with_buffer(n, DEFAULT_OUTPUT.to_owned());

        (
            Self {
                inner: Some(inner),
                named_inners: Default::default(),
            },
            rx,
        )
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test() -> (Self, ReceiverStream<Event>) {
        Self::new_with_buffer(100)
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test_finalize(
        status: event::EventStatus,
    ) -> (Self, impl Stream<Item = Event> + Unpin) {
        let (pipe, recv) = Self::new_with_buffer(100);

        // In a source test pipeline, there is no sink to acknowledge events,
        // so we have to add a map to the receiver to handle the finalization
        let recv = recv.map(move |mut event| {
            let metadata = event.metadata_mut();
            metadata.update_status(status);
            metadata.update_sources();
            event
        });

        (pipe, recv)
    }

    #[cfg(test)]
    pub fn add_outputs(
        &mut self,
        status: event::EventStatus,
        name: String,
    ) -> impl Stream<Item = Event> + Unpin {
        let (inner, recv) = Inner::new_with_buffer(100, name.clone());
        let recv = recv.map(move |mut event| {
            let metadata = event.metadata_mut();
            metadata.update_status(status);
            metadata.update_sources();
            event
        });
        self.named_inners.insert(name, inner);
        recv
    }

    pub async fn send(&mut self, event: Event) -> Result<(), ClosedError> {
        self.inner
            .as_mut()
            .expect("no default output")
            .send(event)
            .await
    }

    pub async fn send_named(&mut self, name: &str, event: Event) -> Result<(), ClosedError> {
        self.named_inners
            .get_mut(name)
            .expect("unknown output")
            .send(event)
            .await
    }

    pub async fn send_all(
        &mut self,
        events: impl Stream<Item = Event> + Unpin,
    ) -> Result<(), ClosedError> {
        self.inner
            .as_mut()
            .expect("no default output")
            .send_all(events)
            .await
    }

    pub async fn send_batch<E, I>(&mut self, events: I) -> Result<(), ClosedError>
    where
        E: Into<Event> + ByteSizeOf,
        I: IntoIterator<Item = E>,
    {
        self.inner
            .as_mut()
            .expect("no default output")
            .send_batch(events)
            .await
    }

    pub async fn send_result_stream<E>(
        &mut self,
        stream: impl Stream<Item = Result<Event, E>> + Unpin,
    ) -> Result<(), StreamSendError<E>> {
        self.inner
            .as_mut()
            .expect("no default output")
            .send_result_stream(stream)
            .await
    }
}

#[derive(Clone, Debug)]
struct Inner {
    inner: mpsc::Sender<Event>,
    output: String,
}

impl Inner {
    fn new_with_buffer(n: usize, output: String) -> (Self, ReceiverStream<Event>) {
        let (tx, rx) = mpsc::channel(n);
        let rx = tokio_stream::wrappers::ReceiverStream::new(rx);
        (Self { inner: tx, output }, ReceiverStream::new(rx))
    }

    async fn send(&mut self, event: Event) -> Result<(), ClosedError> {
        // TODO: add metric
        // let byte_size = event.size_of();
        self.inner.send(event).await?;
        Ok(())
    }

    async fn send_all(
        &mut self,
        events: impl Stream<Item = Event> + Unpin,
    ) -> Result<(), ClosedError> {
        let mut stream = events.ready_chunks(CHUNK_SIZE);
        while let Some(events) = stream.next().await {
            self.send_batch(events).await?;
        }

        Ok(())
    }

    async fn send_batch<E, I>(&mut self, events: I) -> Result<(), ClosedError>
    where
        E: Into<Event> + ByteSizeOf,
        I: IntoIterator<Item = E>,
    {
        let mut count = 0;
        let mut byte_size = 0;

        for event in events.into_iter() {
            let event_size = event.size_of();
            match self.inner.send(event.into()).await {
                Ok(()) => {
                    count += 1;
                    byte_size += event_size;
                }
                Err(err) => {
                    trace!(
                        message = "Events send",
                        %count,
                        %byte_size
                    );

                    return Err(err.into());
                }
            }
        }

        emit!(&EventsSent {
            count,
            byte_size,
            output: Some(&self.output)
        });

        Ok(())
    }

    async fn send_result_stream<E>(
        &mut self,
        mut stream: impl Stream<Item = Result<Event, E>> + Unpin,
    ) -> Result<(), StreamSendError<E>> {
        let mut to_forward = Vec::with_capacity(CHUNK_SIZE);

        loop {
            tokio::select! {
                next = stream.next(), if to_forward.len() <= CHUNK_SIZE => {
                    match next {
                        Some(Ok(event)) => {
                            to_forward.push(event);
                        },
                        Some(Err(err)) => {
                            if !to_forward.is_empty() {
                                self.send_batch(to_forward).await?;
                            }

                            return Err(StreamSendError::Stream(err));
                        },
                        None => {
                            if !to_forward.is_empty() {
                                self.send_batch(to_forward).await?;
                            }

                            break;
                        }
                    }
                }

                else => {
                    if !to_forward.is_empty() {
                        let out = std::mem::replace(&mut to_forward, Vec::with_capacity(CHUNK_SIZE));
                        self.send_batch(out).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[pin_project]
#[derive(Debug)]
pub struct ReceiverStream<T> {
    #[pin]
    inner: tokio_stream::wrappers::ReceiverStream<T>,
}

impl<T> ReceiverStream<T> {
    const fn new(inner: tokio_stream::wrappers::ReceiverStream<T>) -> Self {
        Self { inner }
    }

    pub fn close(&mut self) {
        self.inner.close()
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
