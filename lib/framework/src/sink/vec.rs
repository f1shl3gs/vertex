use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::stream::Peekable;
use futures::{Sink, SinkExt, Stream, StreamExt, ready};

impl<T: ?Sized, Item> VecSinkExt<Item> for T where T: Sink<Item> {}

pub trait VecSinkExt<Item>: Sink<Item> {
    /// A future that completes after the given stream has been fully processed
    /// into the sink, including flushing.
    /// Compare to `SinkExt::send_all` this future accept `Peekable` stream and
    /// do not have own buffer.
    fn send_all_peekable<'a, St>(
        &'a mut self,
        stream: &'a mut Peekable<St>,
    ) -> SendAll<'a, Self, St>
    where
        St: Stream<Item = Item> + Sized,
        Self: Sized,
    {
        SendAll { sink: self, stream }
    }
}

/// Future for the [`send_all_peekable`](VecSinkExt::send_all_peekable) method.
pub struct SendAll<'a, Si, St>
where
    St: Stream,
{
    sink: &'a mut Si,
    stream: &'a mut Peekable<St>,
}

impl<Si, St, Item, Error> Future for SendAll<'_, Si, St>
where
    Si: Sink<Item, Error = Error> + Unpin,
    St: Stream<Item = Item> + Unpin,
{
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match Pin::new(&mut *self.stream).as_mut().poll_peek(cx) {
                Poll::Ready(Some(_)) => {
                    ready!(self.sink.poll_ready_unpin(cx))?;
                    let item = match self.stream.poll_next_unpin(cx) {
                        Poll::Ready(Some(item)) => item,
                        _ => panic!("Item should exist after poll_peek succeeds"),
                    };
                    self.sink.start_send_unpin(item)?;
                }
                Poll::Ready(None) => {
                    ready!(self.sink.poll_flush_unpin(cx))?;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => {
                    ready!(self.sink.poll_flush_unpin(cx))?;
                    return Poll::Pending;
                }
            }
        }
    }
}
