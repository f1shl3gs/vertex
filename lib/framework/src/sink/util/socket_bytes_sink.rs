use std::io::{Error as IoError, ErrorKind};
use std::marker::Unpin;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use codec::BytesCodec;
use event::{EventFinalizers, EventStatus};
use futures::{ready, Sink};
use pin_project_lite::pin_project;
use tokio::io::AsyncWrite;
use tokio_util::codec::FramedWrite;

use super::SocketMode;
use crate::batch::EncodedEvent;

const MAX_PENDING_ITEMS: usize = 1024;

pub enum ShutdownCheck {
    Error(IoError),
    Close(&'static str),
    Alive,
}

struct State {
    socket_mode: SocketMode,
    events_total: usize,
    event_bytes: usize,
    bytes_total: usize,
    finalizers: Vec<EventFinalizers>,
}

impl State {
    fn ack(&mut self, status: EventStatus) {
        if self.events_total > 0 {
            for finalizer in std::mem::take(&mut self.finalizers) {
                finalizer.update_status(status);
            }

            if status == EventStatus::Delivered {
                trace!(
                    message = "Events sent",
                    proto = self.socket_mode.as_str(),
                    count = self.events_total,
                    bytes = self.event_bytes,
                );
                trace!(
                    message = "Bytes sent",
                    proto = self.socket_mode.as_str(),
                    bytes = self.bytes_total
                );
            }

            self.events_total = 0;
            self.event_bytes = 0;
            self.bytes_total = 0;
        }
    }
}

pin_project! {
    /// [FramedWrite](https://docs.rs/tokio-util/0.3.1/tokio_util/codec/struct.FramedWrite.html) wrapper.
    /// Wrapper acts like [Sink](https://docs.rs/futures/0.3.7/futures/sink/trait.Sink.html) forwarding all
    /// calls to `FramedWrite`, but in addition:
    /// - Call `shutdown_check` on each `poll_flush`, so we can stop sending data if other side disconnected.
    /// - Flush all data on each `poll_ready` if total number of events in queue more than some limit.
    /// - Count event size on each `start_send`.
    /// - Ack all sent events on successful `poll_flush` and `poll_close` or on `Drop`.
    pub struct BytesSink<T>
    where
        T: AsyncWrite,
        T: Unpin,
    {
        #[pin]
        inner: FramedWrite<T, BytesCodec>,

        shutdown_check: Box<dyn Fn(&mut T) -> ShutdownCheck + Send>,
        state: State,
    }

    impl<T> PinnedDrop for BytesSink<T>
    where
        T: AsyncWrite,
        T: Unpin,
    {
        fn drop(this: Pin<&mut Self>) {
            this.get_mut().state.ack(EventStatus::Dropped);
        }
    }
}

impl<T> BytesSink<T>
where
    T: AsyncWrite + Unpin,
{
    pub(crate) fn new(
        inner: T,
        shutdown_check: impl Fn(&mut T) -> ShutdownCheck + Send + 'static,
        socket_mode: SocketMode,
    ) -> Self {
        Self {
            inner: FramedWrite::new(inner, BytesCodec::new()),
            shutdown_check: Box::new(shutdown_check),
            state: State {
                events_total: 0,
                event_bytes: 0,
                bytes_total: 0,
                socket_mode,
                finalizers: vec![],
            },
        }
    }
}

impl<T> Sink<EncodedEvent<Bytes>> for BytesSink<T>
where
    T: AsyncWrite + Unpin,
{
    type Error = <FramedWrite<T, BytesCodec> as Sink<Bytes>>::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.as_mut().project().state.events_total >= MAX_PENDING_ITEMS {
            if let Err(err) = ready!(self.as_mut().poll_flush(cx)) {
                return Poll::Ready(Err(err));
            }
        }

        let inner = self.project().inner;
        <FramedWrite<T, BytesCodec> as Sink<Bytes>>::poll_ready(inner, cx)
    }

    fn start_send(self: Pin<&mut Self>, item: EncodedEvent<Bytes>) -> Result<(), Self::Error> {
        let pinned = self.project();
        pinned.state.finalizers.push(item.finalizers);
        pinned.state.events_total += 1;
        pinned.state.event_bytes += item.byte_size;
        pinned.state.bytes_total += item.item.len();

        let result = pinned.inner.start_send(item.item);
        if result.is_err() {
            pinned.state.ack(EventStatus::Errored);
        }

        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let pinned = self.as_mut().project();

        match (pinned.shutdown_check)(pinned.inner.get_mut().get_mut()) {
            ShutdownCheck::Error(err) => return Poll::Ready(Err(err)),
            ShutdownCheck::Close(reason) => {
                if let Err(err) = ready!(self.as_mut().poll_close(cx)) {
                    return Poll::Ready(Err(err));
                }

                return Poll::Ready(Err(IoError::new(ErrorKind::Other, reason)));
            }
            ShutdownCheck::Alive => {}
        }

        let inner = self.as_mut().project().inner;
        let result = ready!(<FramedWrite<T, BytesCodec> as Sink<Bytes>>::poll_flush(
            inner, cx
        ));
        self.as_mut().get_mut().state.ack(match result {
            Ok(_) => EventStatus::Delivered,
            Err(_) => EventStatus::Errored,
        });
        Poll::Ready(result)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let inner = self.as_mut().project().inner;
        let result = ready!(<FramedWrite<T, BytesCodec> as Sink<Bytes>>::poll_close(
            inner, cx
        ));
        self.as_mut().get_mut().state.ack(EventStatus::Dropped);
        Poll::Ready(result)
    }
}

mod codec {
    use bytes::{BufMut, Bytes, BytesMut};
    /// I can't figure out how to fix the compile error. So i just copy the `BytesCodec` to this file
    ///
    ///```text
    /// error[E0283]: type annotations needed
    ///    --> lib/framework/src/sink/util/socket_bytes_sink.rs:104:15
    ///     |
    /// 104 |         inner.poll_ready(cx)
    ///     |               ^^^^^^^^^^ cannot infer type for type parameter `I`
    ///     |
    ///     = note: multiple `impl`s satisfying `BytesCodec: tokio_util::codec::Encoder<_>` found in the `tokio_util` crate:
    ///             - impl tokio_util::codec::Encoder<BytesMut> for BytesCodec;
    ///             - impl tokio_util::codec::Encoder<bytes::Bytes> for BytesCodec;
    ///     = note: required because of the requirements on the impl of `futures::Sink<_>` for `FramedWrite<T, BytesCodec>`
    ///```
    use std::io;
    use tokio_util::codec::{Decoder, Encoder};

    #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
    pub struct BytesCodec(());

    impl BytesCodec {
        /// Creates a new `BytesCodec` for shipping around raw bytes.
        pub fn new() -> BytesCodec {
            BytesCodec(())
        }
    }

    impl Decoder for BytesCodec {
        type Item = BytesMut;
        type Error = io::Error;

        fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<BytesMut>, io::Error> {
            if !buf.is_empty() {
                let len = buf.len();
                Ok(Some(buf.split_to(len)))
            } else {
                Ok(None)
            }
        }
    }

    impl Encoder<Bytes> for BytesCodec {
        type Error = io::Error;

        fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> Result<(), io::Error> {
            buf.reserve(data.len());
            buf.put(data);
            Ok(())
        }
    }
}
