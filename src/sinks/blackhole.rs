use crate::config::{SinkConfig, SinkContext, DataType};
use crate::sinks::Sink;
use async_trait::async_trait;
use crate::event::Event;
use std::pin::Pin;
use std::task::{Context, Poll};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlackholeConfig {
    pub rate: Option<usize>
}

#[async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for BlackholeConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<Sink> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        todo!()
    }

    fn sink_type(&self) -> &'static str {
        todo!()
    }
}

struct Blackhole {}

impl futures::Sink<Event> for Blackhole {
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: Pin<&mut Self>, item: Event) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}