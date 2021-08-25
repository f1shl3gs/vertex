use futures::channel::mpsc;
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
    outstanding: usize,
}

impl Pipeline {
    fn try_flush(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), <Self as futures::Sink<Event>>::Error>> {
        // We batch the updates to "events out" for efficiency, and do it
        // here because it gives us a chance to allow the natural batching
        // of `Pipeline` to kick in
        if self.outstanding > 0 {
            self.outstanding = 0;
        }

        while let Some(event) = self.enqueued.pop_front() {
            match self.inner.poll_ready(cx) {
                Poll::Pending => {
                    self.enqueued.push_front(event);
                    return Poll::Pending;
                }

                Poll::Ready(Ok(())) => {
                    // continue to send blow
                }

                Poll::Ready(Err(_err)) => return Poll::Ready(Err(ClosedError)),
            }

            match self.inner.start_send(event) {
                Ok(()) => {
                    // we good, keep looping
                }

                Err(err) if err.is_full() => {
                    // We only try to send after a successful call to poll_ready,
                    // which reserves space for us in the channel. That makes this
                    // branch unreachable as long as the channel implementation fulfills
                    // its own contract.
                    panic!("Channel was both ready and full; this is a bug")
                }

                Err(err) if err.is_disconnected() => {
                    return Poll::Ready(Err(ClosedError));
                }

                Err(_) => unreachable!()
            }
        }

        Poll::Ready(Ok(()))
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
            outstanding: 0,
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

    fn start_send(mut self: Pin<&mut Self>, item: Event) -> Result<(), Self::Error> {
        self.outstanding += 1;
        // Note how this gets **swapped** with `new_working_set` in the loop.
        // At the end of the loop, it will only contain finalized events.
        let mut working_set = vec![item];
        for inline in self.inlines.iter_mut() {
            let mut new_working_set = Vec::with_capacity(working_set.len());
            for event in working_set.drain(..) {
                inline.transform(&mut new_working_set, event);
            }

            core::mem::swap(&mut new_working_set, &mut working_set);
        }
        self.enqueued.extend(working_set);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.try_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}