use std::fmt::Debug;
use std::pin::Pin;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::task::{Context, Poll};

use event::{DecodeBytes, EncodeBytes};
use futures::Sink;

use crate::usage::BufferUsageData;

pub struct Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    offset: Arc<AtomicUsize>,

    slot: Option<T>,

    usage: Arc<BufferUsageData>,
}

impl<T> Clone for Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    fn clone(&self) -> Self {
        Self {
            offset: self.offset.clone(),
            slot: None,
            usage: self.usage.clone(),
        }
    }
}

impl<T> Sink<T> for Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: Pin<&mut Self>, _item: T) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

impl<T> Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    fn flush(&self) {
        // TODO
    }
}

impl<T> Drop for Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    fn drop(&mut self) {
        // TODO
    }
}
