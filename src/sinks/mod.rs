#[cfg(feature = "sinks-blackhole")]
mod blackhole;
#[cfg(feature = "sinks-kafka")]
mod kafka;
#[cfg(feature = "sinks-pulsar")]
mod pulsar;
#[cfg(feature = "sinks-prometheus_exporter")]
mod prometheus_exporter;
#[cfg(feature = "sinks-stdout")]
mod stdout;
#[cfg(feature = "sinks-elasticsearch")]
mod elasticsearch;
#[cfg(feature = "sinks-loki")]
mod loki;
#[cfg(feature = "sinks-vertex")]
mod vertex;
#[cfg(feature = "sinks-clickhouse")]
mod clickhouse;

mod util;


use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{Stream, StreamExt};
use std::fmt::{Debug, Formatter};
use event::Event;

#[async_trait]
pub trait StreamSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()>;
}

pub enum Sink {
    Sink(Box<dyn futures::Sink<Event, Error=()> + Send + Unpin>),
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
            S: Stream<Item=Event> + Send,
    {
        match self {
            Self::Sink(sink) => input.map(Ok).forward(sink).await,
            Self::Stream(s) => s.run(Box::pin(input)).await,
        }
    }

    /// Converts `Sink` into a `futures::Sink`
    ///
    /// # Panics
    ///
    /// This function will panic if the self instance is not `Sink`.
    pub fn into_sink(self) -> Box<dyn futures::Sink<Event, Error=()> + Send + Unpin> {
        match self {
            Self::Sink(sink) => sink,
            _ => panic!("Failed type coercion, {:?} is not a Sink", self)
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
            _ => panic!("Failed type coercion, {:?} is not a Stream", self)
        }
    }
}