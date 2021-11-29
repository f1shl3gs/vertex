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

use event::Event;
use async_trait::async_trait;
use futures::stream::{
    BoxStream,
    Stream,
    StreamExt,
};

#[async_trait]
pub trait StreamSink {
    async fn run(&mut self, input: BoxStream<'_, Event>) -> Result<(), ()>;
}

pub enum Sink {
    Sink(Box<dyn futures::Sink<Event, Error=()> + Send + Unpin>),
    Stream(Box<dyn StreamSink + Send>),
}

impl Sink {
    /// Run the `Sink`
    pub async fn run<S>(mut self, input: S) -> Result<(), ()>
        where
            S: Stream<Item=Event> + Send,
    {
        match self {
            Self::Sink(sink) => input.map(Ok).forward(sink).await,
            Self::Stream(ref mut s) => s.run(Box::pin(input)).await,
        }
    }
}