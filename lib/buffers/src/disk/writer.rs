use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use futures::Sink;
use crate::{DecodeBytes, EncodeBytes};

pub struct Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T>  + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    // TODO
}

impl<T> Clone for Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{
    fn clone(&self) -> Self {
        Self {
            // TODO
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

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        todo!()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
    }
}

impl<T> Writer<T>
where
    T: Send + Sync + Unpin + EncodeBytes<T> + DecodeBytes<T>,
    <T as EncodeBytes<T>>::Error: Debug,
    <T as DecodeBytes<T>>::Error: Debug,
{

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