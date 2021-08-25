use tokio::sync::mpsc;
use crate::event::Event;
use std::collections::VecDeque;
use std::pin::Pin;
use std::{
    fmt,
    task::{Context, Poll},
};
use crate::transforms::FunctionTransform;

#[derive(Debug)]
pub struct ClosedError;

impl fmt::Display for ClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pipeline is closed.")
    }
}

impl std::error::Error for ClosedError {}

const MAX_ENQUEUED: usize = 1024;

pub struct Pipeline {
    inner: mpsc::Sender<Event>,
    enqueued: VecDeque<Event>,

    inlines: Vec<Box<dyn FunctionTransform>>,
}

impl Pipeline {
    fn try_flush(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), <Self as futures::Sink<Event>>::Error>> {
        todo!()
    }

    pub fn from_sender(
        inner: mpsc::Sender<Event>,
        inlines: Vec<Box<dyn FunctionTransform>>,
    ) -> Self {
        Self {
            inner,
            inlines,
            // We ensure the buffer is sufficient that it is unlikely to
            // require re-allocations. There is a possibility a component
            // might blow this queue size.
            enqueued: VecDeque::with_capacity(16),
        }
    }
}

impl futures::Sink<Event> for Pipeline {
    type Error = ClosedError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.enqueued.len() < MAX_ENQUEUED {
            Poll::Ready(Ok(()))
        } else {
            self.try_flush(cx)
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Event) -> Result<(), Self::Error> {
        todo!()
        // Note how this gets **swapped** with `new_working_set` in the loop.
        // At the end of the loop, it will only contain finalized events.
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}