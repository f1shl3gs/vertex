pub mod http;
pub mod socket_bytes_sink;
pub mod tcp;
pub mod udp;
#[cfg(unix)]
pub mod unix;
pub mod util;

mod vec;

// Re-export
pub use vec::{SendAll, VecSinkExt};

use std::fmt::{Debug, Formatter};
use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use event::{Event, EventContainer, Events};
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{SinkExt, Stream, StreamExt};
use thiserror::Error;

pub type Healthcheck = BoxFuture<'static, crate::Result<()>>;

/// Common healthcheck errors
#[derive(Debug, Error)]
pub enum HealthcheckError {
    #[error("Unexpected status: {0}, {1}")]
    UnexpectedStatus(::http::StatusCode, String),
}

#[async_trait]
pub trait StreamSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()>;
}

pub enum Sink {
    Sink(Box<dyn futures::Sink<Events, Error = ()> + Send + Unpin>),
    Stream(Box<dyn StreamSink + Send>),
}

impl Debug for Sink {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sink").finish()
    }
}

impl Sink {
    /// Run the `Sink`
    ///
    /// # Errors
    ///
    /// It is unclear under what conditions this function will error.
    pub async fn run<S>(self, input: S) -> Result<(), ()>
    where
        S: Stream<Item = Events> + Send,
    {
        match self {
            Self::Sink(sink) => input.map(Ok).forward(sink).await,
            Self::Stream(s) => s.run(Box::pin(input)).await,
        }
    }

    /// Run the `Sink` with a one-time `Vec` of `Event`, for use in tests
    ///
    /// Note: this function should be used for test only.
    pub async fn run_events<I>(self, input: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = Event> + Send,
        I::IntoIter: Send,
    {
        self.run(futures::stream::iter(input).map(Into::into)).await
    }

    /// Converts `Sink` into a `futures::Sink`
    ///
    /// # Panics
    ///
    /// This function will panic if the self instance is not `Sink`.
    pub fn into_sink(self) -> Box<dyn futures::Sink<Events, Error = ()> + Send + Unpin> {
        match self {
            Self::Sink(sink) => sink,
            _ => panic!("Failed type coercion, {:?} is not a Sink", self),
        }
    }

    /// Converts `Sink` into a `StreamSink`
    ///
    /// # Panics
    ///
    /// This function will panic if the self instance is not `Sink`.
    pub fn into_stream(self) -> Box<dyn StreamSink + Send> {
        match self {
            Self::Stream(stream) => stream,
            _ => panic!("Failed type coercion, {:?} is not a Stream", self),
        }
    }

    pub fn from_event_sink(
        sink: impl futures::Sink<Event, Error = ()> + Send + Unpin + 'static,
    ) -> Self {
        Sink::Sink(Box::new(EventSink::new(sink)))
    }
}

/// Wrapper for sinks implementing `Sink<Event>` to implement
/// `Sink<Events>`. This stores an iterator over the incoming
/// `Events` to be pushed into the wrapped sink one at a time.
///
/// This should be removed once the sinks are all refactored to be consume
/// `Events` rather than `Event`.
struct EventSink<S> {
    sink: S,
    queue: Option<<Events as EventContainer>::IntoIter>,
}

macro_rules! poll_ready_ok {
    ( $e:expr ) => {
        match $e {
            r @ (Poll::Pending | Poll::Ready(Err(_))) => return r,
            Poll::Ready(Ok(ok)) => ok,
        }
    };
}

impl<S: futures::Sink<Event> + Send + Unpin> EventSink<S> {
    fn new(sink: S) -> Self {
        Self { sink, queue: None }
    }

    fn next_event(&mut self) -> Option<Event> {
        match &mut self.queue {
            #[allow(clippy::single_match_else)] // No, clippy, this isn't a single pattern
            Some(queue) => match queue.next() {
                Some(event) => Some(event),
                None => {
                    // Reset the queue to empty after the last event
                    self.queue = None;
                    None
                }
            },
            None => None,
        }
    }

    fn flush_queue(self: &mut Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        while self.queue.is_some() {
            poll_ready_ok!(self.sink.poll_ready_unpin(cx));
            let event = match self.next_event() {
                None => break,
                Some(event) => event,
            };
            if let Err(err) = self.sink.start_send_unpin(event) {
                return Poll::Ready(Err(err));
            }
        }
        Poll::Ready(Ok(()))
    }
}

impl<S: futures::Sink<Event> + Send + Unpin> futures::Sink<Events> for EventSink<S> {
    type Error = S::Error;
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        poll_ready_ok!(self.flush_queue(cx));
        self.sink.poll_ready_unpin(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, events: Events) -> Result<(), Self::Error> {
        assert!(self.queue.is_none()); // Should be guaranteed by `poll_ready`
        self.queue = Some(events.into_events());
        self.next_event()
            .map_or(Ok(()), |event| self.sink.start_send_unpin(event))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        poll_ready_ok!(self.flush_queue(cx));
        self.sink.poll_flush_unpin(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        poll_ready_ok!(self.flush_queue(cx));
        self.sink.poll_close_unpin(cx)
    }
}
