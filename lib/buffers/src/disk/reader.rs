use std::fmt::Display;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use event::DecodeBytes;
use futures::Stream;

pub struct Reader<T>
where
    T: Send + Sync + Unpin,
{
    phantom: PhantomData<T>,
}

impl<T> Stream for Reader<T>
where
    T: Send + Sync + Unpin + DecodeBytes<T>,
    <T as DecodeBytes<T>>::Error: Display,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!()
    }
}

impl<T> Drop for Reader<T>
where
    T: Send + Sync + Unpin,
{
    fn drop(&mut self) {
        self.flush()
    }
}

impl<T> Reader<T>
where
    T: Send + Sync + Unpin,
{
    fn flush(&mut self) {
        // TODO
    }

    fn compact(&mut self) {
        // TODO
    }
}
