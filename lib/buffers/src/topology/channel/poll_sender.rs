use std::{
    fmt, mem,
    task::{Context, Poll},
};

use tokio::sync::mpsc::{OwnedPermit, Sender};
use tokio_util::sync::ReusableBoxFuture;

// NOTE: `PollSender<T>` has been directly vendored here via copy/paste due to issues with
// overriding a single crate like `tokio-util` and having it spiral out in a way that causes issues
// like "perhaps two different versions of crate `tokio` are being used?".
//
// Once our upstream PR (https://github.com/tokio-rs/tokio/pull/4214) is accepted and a new version
// is cut, we can switch to using `PollSender<T>` from `tokio-util` directly.

/// Error returned by `PollSender<T>` when the channel is closed.
#[derive(Debug)]
pub struct PollSendError<T>(pub(crate) Option<T>);

impl<T> PollSendError<T> {
    /// Consumes the stored value, if any.
    ///
    /// If this error was encountered when calling `start_send`, this will be the item that the
    /// caller attempted to send.  Otherwise, it will be `None`.
    pub fn into_inner(self) -> Option<T> {
        self.0
    }
}

impl<T> fmt::Display for PollSendError<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "channel closed")
    }
}

impl<T: fmt::Debug> std::error::Error for PollSendError<T> {}

#[derive(Debug)]
enum State<T> {
    Idle(Sender<T>),
    Acquiring,
    ReadyToSend(OwnedPermit<T>),
    Closed,
}

/// A wrapper around [`mpsc::Sender`] that can be polled.
///
/// [`mpsc::Sender`]: tokio::sync::mpsc::Sender
#[derive(Debug)]
pub struct PollSender<T> {
    sender: Option<Sender<T>>,
    state: State<T>,
    acquire: ReusableBoxFuture<Result<OwnedPermit<T>, PollSendError<T>>>,
}

// Creates a future for acquiring a permit from the underlying channel.  This is used to ensure
// there's capacity for a send to complete.
//
// By reusing the same async fn for both `Some` and `None`, we make sure every future passed to
// ReusableBoxFuture has the same underlying type, and hence the same size and alignment.
async fn make_acquire_future<T>(
    data: Option<Sender<T>>,
) -> Result<OwnedPermit<T>, PollSendError<T>> {
    match data {
        Some(sender) => sender
            .reserve_owned()
            .await
            .map_err(|_| PollSendError(None)),
        None => unreachable!("this future should not be pollable in this state"),
    }
}

impl<T: Send + 'static> PollSender<T> {
    /// Creates a new `PollSender`.
    pub fn new(sender: Sender<T>) -> Self {
        Self {
            sender: Some(sender.clone()),
            state: State::Idle(sender),
            acquire: ReusableBoxFuture::new(make_acquire_future(None)),
        }
    }

    fn take_state(&mut self) -> State<T> {
        mem::replace(&mut self.state, State::Closed)
    }

    /// Attempts to prepare the sender to receive a value.
    ///
    /// This method must be called and return `Poll::Ready(Ok(()))` prior to each call to
    /// `start_send`.
    ///
    /// This method returns `Poll::Ready` once the underlying channel is ready to receive a value,
    /// by reserving a slot in the channel for the item to be sent. If this method returns
    /// `Poll::Pending`, the current task is registered to be notified (via
    /// `cx.waker().wake_by_ref()`) when `poll_reserve` should be called again.
    ///
    /// # Errors
    ///
    /// If the channel is closed, an error will be returned.  This is a permanent state.
    pub fn poll_reserve(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), PollSendError<T>>> {
        loop {
            let (result, next_state) = match self.take_state() {
                State::Idle(sender) => {
                    // Start trying to acquire a permit to reserve a slot for our send, and
                    // immediately loop back around to poll it the first time.
                    self.acquire.set(make_acquire_future(Some(sender)));
                    (None, State::Acquiring)
                }
                State::Acquiring => match self.acquire.poll(cx) {
                    // Channel has capacity.
                    Poll::Ready(Ok(permit)) => {
                        (Some(Poll::Ready(Ok(()))), State::ReadyToSend(permit))
                    }
                    // Channel is closed.
                    Poll::Ready(Err(e)) => (Some(Poll::Ready(Err(e))), State::Closed),
                    // Channel doesn't have capacity yet, so we need to wait.
                    Poll::Pending => (Some(Poll::Pending), State::Acquiring),
                },
                // We're closed, either by choice or because the underlying sender was closed.
                s @ State::Closed => (Some(Poll::Ready(Err(PollSendError(None)))), s),
                // We're already ready to send an item.
                s @ State::ReadyToSend(_) => (Some(Poll::Ready(Ok(()))), s),
            };

            self.state = next_state;
            if let Some(result) = result {
                return result;
            }
        }
    }

    /// Sends an item to the channel.
    ///
    /// Before calling `start_send`, `poll_reserve` must be called with a successful return
    /// value of `Poll::Ready(Ok(()))`.
    ///
    /// # Errors
    ///
    /// If the channel is closed, an error will be returned.  This is a permanent state.
    ///
    /// # Panics
    ///
    /// If `poll_reserve` was not successfully called prior to calling `start_send`, then this method
    /// will panic.
    pub fn start_send(&mut self, value: T) -> Result<(), PollSendError<T>> {
        let (result, next_state) = match self.take_state() {
            State::Idle(_) | State::Acquiring => {
                panic!("`start_send` called without first calling `poll_reserve`")
            }
            // We have a permit to send our item, so go ahead, which gets us our sender back.
            State::ReadyToSend(permit) => (Ok(()), State::Idle(permit.send(value))),
            // We're closed, either by choice or because the underlying sender was closed.
            State::Closed => (Err(PollSendError(Some(value))), State::Closed),
        };

        // Handle deferred closing if `close` was called between `poll_reserve` and `start_send`.
        self.state = if self.sender.is_some() {
            next_state
        } else {
            State::Closed
        };
        result
    }

    /// Checks whether this sender is been closed.
    ///
    /// The underlying channel that this sender was wrapping may still be open.
    pub fn is_closed(&self) -> bool {
        matches!(self.state, State::Closed) || self.sender.is_none()
    }

    /// Gets a reference to the underlying `Sender`.
    ///
    /// If the channel is closed, `None` is returned.
    pub fn get_ref(&self) -> Option<&Sender<T>> {
        self.sender.as_ref()
    }

    /// Closes this sender without dropping it.
    ///
    /// No more messages will be able to be sent from this sender, but the underlying channel will
    /// remain open until all senders have dropped, or until the [`Receiver`] closes the channel.
    ///
    /// If a slot was previously reserved by calling `poll_reserve`, then a final call can be made
    /// to `start_send` in order to consume the reserved slot.  After that, no further sends will be
    /// possible.  If you do not intend to send another item, you can release the reserved slot back
    /// to the underlying sender by calling [`abort_send`].
    ///
    /// [`abort_send`]: crate::sync::PollSender::abort_send
    /// [`Receiver`]: tokio::sync::mpsc::Receiver
    pub fn close(&mut self) {
        // Mark ourselves officially closed by dropping our main sender.
        self.sender = None;

        // If we're already idle, closed, or we haven't yet reserved a slot, we can quickly
        // transition to the closed state.  Otherwise, leave the existing permit in place for the
        // caller if they want to complete the send.
        match self.state {
            State::Idle(_) => self.state = State::Closed,
            State::Acquiring => {
                self.acquire.set(make_acquire_future(None));
                self.state = State::Closed;
            }
            _ => {}
        }
    }

    /// Aborts the current in-progress send, if any.
    ///
    /// Returns `true` if a send was aborted.  If the sender was closed prior to calling
    /// `abort_send`, then the sender will remain in the closed state, otherwise the sender will be
    /// ready to attempt another send.
    pub fn abort_send(&mut self) -> bool {
        // We may have been closed in the meantime, after a call to `poll_reserve` already
        // succeeded.  We'll check if `self.sender` is `None` to see if we should transition to the
        // closed state when we actually abort a send, rather than resetting ourselves back to idle.

        let (result, next_state) = match self.take_state() {
            // We're currently trying to reserve a slot to send into.
            State::Acquiring => {
                // Replacing the future drops the in-flight one.
                self.acquire.set(make_acquire_future(None));

                // If we haven't closed yet, we have to clone our stored sender since we have no way
                // to get it back from the acquire future we just dropped.
                let state = match self.sender.clone() {
                    Some(sender) => State::Idle(sender),
                    None => State::Closed,
                };
                (true, state)
            }
            // We got the permit.  If we haven't closed yet, get the sender back.
            State::ReadyToSend(permit) => {
                let state = if self.sender.is_some() {
                    State::Idle(permit.release())
                } else {
                    State::Closed
                };
                (true, state)
            }
            s => (false, s),
        };

        self.state = next_state;
        result
    }
}

impl<T> Clone for PollSender<T> {
    /// Clones this `PollSender`.
    ///
    /// The resulting `PollSender` will have an initial state identical to calling `PollSender::new`.
    fn clone(&self) -> PollSender<T> {
        let (sender, state) = match self.sender.clone() {
            Some(sender) => (Some(sender.clone()), State::Idle(sender)),
            None => (None, State::Closed),
        };

        Self {
            sender,
            state,
            // We don't use `make_acquire_future` here because our relaxed bounds on `T` are not
            // compatible with the transitive bounds required by `Sender<T>`.
            acquire: ReusableBoxFuture::new(async { unreachable!() }),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::future::poll_fn;
    use tokio::sync::mpsc::channel;
    use tokio_test::{
        assert_pending, assert_ready, assert_ready_err, assert_ready_ok, task::spawn,
    };

    use super::PollSender;

    #[tokio::test]
    async fn simple() {
        let (send, mut recv) = channel(3);
        let mut send = PollSender::new(send);

        for i in 1..=3_i32 {
            let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
            assert_ready_ok!(reserve.poll());
            send.start_send(i).unwrap();
        }

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_pending!(reserve.poll());

        assert_eq!(recv.recv().await.unwrap(), 1);
        assert!(reserve.is_woken());
        assert_ready_ok!(reserve.poll());

        drop(recv);

        send.start_send(42).unwrap();
    }

    #[tokio::test]
    async fn repeated_poll_reserve() {
        let (send, mut recv) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        assert_ready_ok!(reserve.poll());
        send.start_send(1).unwrap();

        assert_eq!(recv.recv().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn abort_send() {
        let (send, mut recv) = channel(3);
        let mut send = PollSender::new(send);
        let send2 = send.get_ref().cloned().unwrap();

        for i in 1..=3_i32 {
            let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
            assert_ready_ok!(reserve.poll());
            send.start_send(i).unwrap();
        }

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_pending!(reserve.poll());
        assert_eq!(recv.recv().await.unwrap(), 1);
        assert!(reserve.is_woken());
        assert_ready_ok!(reserve.poll());

        let mut send2_send = spawn(send2.send(5));
        assert_pending!(send2_send.poll());
        assert!(send.abort_send());
        assert!(send2_send.is_woken());
        assert_ready_ok!(send2_send.poll());

        assert_eq!(recv.recv().await.unwrap(), 2);
        assert_eq!(recv.recv().await.unwrap(), 3);
        assert_eq!(recv.recv().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn close_sender_last() {
        let (send, mut recv) = channel::<i32>(3);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        send.close();

        assert!(recv_task.is_woken());
        assert!(assert_ready!(recv_task.poll()).is_none());
    }

    #[tokio::test]
    async fn close_sender_not_last() {
        let (send, mut recv) = channel::<i32>(3);
        let mut send = PollSender::new(send);
        let send2 = send.get_ref().cloned().unwrap();

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        send.close();

        assert!(!recv_task.is_woken());
        assert_pending!(recv_task.poll());

        drop(send2);

        assert!(recv_task.is_woken());
        assert!(assert_ready!(recv_task.poll()).is_none());
    }

    #[tokio::test]
    async fn close_sender_before_reserve() {
        let (send, mut recv) = channel::<i32>(3);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        send.close();

        assert!(recv_task.is_woken());
        assert!(assert_ready!(recv_task.poll()).is_none());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_err!(reserve.poll());
    }

    #[tokio::test]
    async fn close_sender_after_pending_reserve() {
        let (send, mut recv) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        send.start_send(1).unwrap();

        assert!(recv_task.is_woken());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_pending!(reserve.poll());
        drop(reserve);

        send.close();

        assert!(send.is_closed());
        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_err!(reserve.poll());
    }

    #[tokio::test]
    async fn close_sender_after_successful_reserve() {
        let (send, mut recv) = channel::<i32>(3);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        drop(reserve);

        send.close();
        assert!(send.is_closed());
        assert!(!recv_task.is_woken());
        assert_pending!(recv_task.poll());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
    }

    #[tokio::test]
    async fn abort_send_after_pending_reserve() {
        let (send, mut recv) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        send.start_send(1).unwrap();

        assert_eq!(send.get_ref().unwrap().capacity(), 0);
        assert!(!send.abort_send());

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_pending!(reserve.poll());

        assert!(send.abort_send());
        assert_eq!(send.get_ref().unwrap().capacity(), 0);
    }

    #[tokio::test]
    async fn abort_send_after_successful_reserve() {
        let (send, mut recv) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut recv_task = spawn(recv.recv());
        assert_pending!(recv_task.poll());

        assert_eq!(send.get_ref().unwrap().capacity(), 1);
        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        assert_eq!(send.get_ref().unwrap().capacity(), 0);

        assert!(send.abort_send());
        assert_eq!(send.get_ref().unwrap().capacity(), 1);
    }

    #[tokio::test]
    async fn closed_when_receiver_drops() {
        let (send, _) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_err!(reserve.poll());
    }

    #[should_panic]
    #[test]
    fn start_send_panics_when_idle() {
        let (send, _) = channel::<i32>(3);
        let mut send = PollSender::new(send);

        send.start_send(1).unwrap();
    }

    #[should_panic]
    #[test]
    fn start_send_panics_when_acquiring() {
        let (send, _) = channel::<i32>(1);
        let mut send = PollSender::new(send);

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_ready_ok!(reserve.poll());
        send.start_send(1).unwrap();

        let mut reserve = spawn(poll_fn(|cx| send.poll_reserve(cx)));
        assert_pending!(reserve.poll());
        send.start_send(2).unwrap();
    }
}
