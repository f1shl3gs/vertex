use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{Stream, ready};
use tokio::select;
use tokio_util::sync::ReusableBoxFuture;

use super::limited_queue::LimitedReceiver;
use crate::variants::disk::{ProductionFilesystem, Reader};
use crate::{Bufferable, buffer_usage_data::BufferUsageHandle};

/// Adapter for papering over various receiver backends by providing a [`Stream`] interface.
#[derive(Debug)]
pub enum ReceiverAdapter<T> {
    /// A receiver that uses an in-memory channel.
    Memory(LimitedReceiver<T>),

    /// A receiver that provides its own [`Stream`] implementation.
    Disk(Reader<T, ProductionFilesystem>),
}

impl<T: Bufferable> From<LimitedReceiver<T>> for ReceiverAdapter<T> {
    fn from(value: LimitedReceiver<T>) -> Self {
        Self::Memory(value)
    }
}

impl<T: Bufferable> From<Reader<T, ProductionFilesystem>> for ReceiverAdapter<T> {
    fn from(value: Reader<T, ProductionFilesystem>) -> Self {
        Self::Disk(value)
    }
}

impl<T> ReceiverAdapter<T>
where
    T: Bufferable,
{
    pub async fn next(&mut self) -> Option<T> {
        match self {
            ReceiverAdapter::Memory(rx) => rx.next().await,
            ReceiverAdapter::Disk(reader) => loop {
                match reader.next().await {
                    Ok(result) => break result,
                    Err(err) => {
                        if err.is_recoverable_error() {
                            // If we've hit a recoverable error, we'll emit an event to
                            // indicate as much but we'll still keep trying to read the
                            // next available record.
                            continue;
                        }

                        panic!("Reader encountered unrecoverable error: {err:?}");
                    }
                }
            },
        }
    }
}

/// A buffer receiver.
///
/// The receiver handles retrieving events from the buffer, regardless of the overall buffer configuration.
///
/// If a buffer was configured to operate in "overflow" mode, then the receiver will be responsible
/// for querying the overflow buffer as well.  The ordering of events when operating in "overflow"
/// is undefined, as the receiver will try to manage polling both its own buffer, as well as the
/// overflow buffer, in order to fairly balance throughput.
#[derive(Debug)]
pub struct BufferReceiver<T> {
    base: ReceiverAdapter<T>,
    overflow: Option<Box<BufferReceiver<T>>>,
    instrumentation: Option<BufferUsageHandle>,
}

impl<T: Bufferable> BufferReceiver<T> {
    /// Creates a new [`BufferReceiver`] wrapping the given channel receiver.
    pub fn new(base: ReceiverAdapter<T>) -> Self {
        Self {
            base,
            overflow: None,
            instrumentation: None,
        }
    }

    /// Creates a new [`BufferReceiver`] wrapping the given channel receiver and overflow receiver.
    pub fn with_overflow(base: ReceiverAdapter<T>, overflow: BufferReceiver<T>) -> Self {
        Self {
            base,
            overflow: Some(Box::new(overflow)),
            instrumentation: None,
        }
    }

    /// Converts this receiver into an overflowing receiver using the given `BufferSender<T>`.
    ///
    /// Note: this resets the internal state of this sender, and so this should not be called except
    /// when initially constructing `BufferSender<T>`.
    #[cfg(test)]
    pub fn switch_to_overflow(&mut self, overflow: BufferReceiver<T>) {
        self.overflow = Some(Box::new(overflow));
    }

    /// Configures this receiver to instrument the items passing through it.
    pub fn with_instrumentation(&mut self, handle: BufferUsageHandle) {
        self.instrumentation = Some(handle);
    }

    pub async fn next(&mut self) -> Option<T> {
        // We want to poll both our base and overflow receivers without waiting for one or the
        // other to entirely drain before checking the other.  This ensures that we're fairly
        // servicing both receivers, and avoiding stalls in one or the other.
        //
        // This is primarily important in situations where an overflow-triggering event has
        // occurred, and is over, and items are flowing through the base receiver.  If we waited to
        // entirely drain the overflow receiver, we might cause another small stall of the pipeline
        // attached to the base receiver.
        let overflow = self.overflow.as_mut().map(Pin::new);

        let (item, from_base) = match overflow {
            None => match self.base.next().await {
                Some(item) => (item, true),
                None => return None,
            },
            Some(mut overflow) => {
                select! {
                    Some(item) = Box::pin(overflow.next()) => (item, false),
                    Some(item) = self.base.next() => (item, true),
                    else => return None,
                }
            }
        };

        // If instrumentation is enabled, and we got the item from the base receiver, then and only
        // then do we track sending the event out.
        if let Some(handle) = self.instrumentation.as_ref() {
            if from_base {
                handle.increment_sent_event_count_and_byte_size(
                    item.event_count() as u64,
                    item.size_of() as u64,
                );
            }
        }

        Some(item)
    }

    pub fn into_stream(self) -> BufferReceiverStream<T> {
        BufferReceiverStream::new(self)
    }
}

enum StreamState<T: Bufferable> {
    Idle(BufferReceiver<T>),
    Polling,

    #[allow(dead_code)]
    Closed(BufferReceiver<T>),
}

pub struct BufferReceiverStream<T: Bufferable> {
    state: StreamState<T>,
    recv_fut: ReusableBoxFuture<'static, (Option<T>, BufferReceiver<T>)>,
}

impl<T: Bufferable> BufferReceiverStream<T> {
    pub fn new(receiver: BufferReceiver<T>) -> Self {
        Self {
            state: StreamState::Idle(receiver),
            recv_fut: ReusableBoxFuture::new(make_recv_future(None)),
        }
    }
}

impl<T: Bufferable> Stream for BufferReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match mem::replace(&mut self.state, StreamState::Polling) {
                s @ StreamState::Closed(_) => {
                    self.state = s;
                    return Poll::Ready(None);
                }
                StreamState::Idle(receiver) => {
                    self.recv_fut.set(make_recv_future(Some(receiver)));
                }
                StreamState::Polling => {
                    let (result, receiver) = ready!(self.recv_fut.poll(cx));
                    self.state = if result.is_none() {
                        StreamState::Closed(receiver)
                    } else {
                        StreamState::Idle(receiver)
                    };

                    return Poll::Ready(result);
                }
            }
        }
    }
}

async fn make_recv_future<T: Bufferable>(
    receiver: Option<BufferReceiver<T>>,
) -> (Option<T>, BufferReceiver<T>) {
    match receiver {
        None => panic!("invalid to poll future in uninitialized state"),
        Some(mut receiver) => {
            let result = receiver.next().await;
            (result, receiver)
        }
    }
}
