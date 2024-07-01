use std::pin::{pin, Pin};
use std::task::{Context, Poll};

use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::signal::unix::Signal;
use tokio::sync::{broadcast, mpsc};

use crate::config::Builder;

pub type ShutdownTx = broadcast::Sender<()>;
pub type SignalTx = mpsc::Sender<SignalTo>;
pub type SignalRx = mpsc::Receiver<SignalTo>;

/// Control messages used by Vertex to drive topology and shutdown events.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SignalTo {
    /// Signal to reload config from a string.
    ReloadFromConfigBuilder(Builder),
    /// Signal to reload config from the filesystem
    ReloadFromDisk,
    /// Signal to shutdown process
    Shutdown,
    /// Shutdown process immediately
    Quit,
}

/// SignalHandler is a general `ControlTo` message receiver and transmitter.
/// It's used by OS signals and providers to surface control events to the
/// root of the application.
pub struct SignalHandler {
    tx: SignalTx,
    shutdown_txs: Vec<ShutdownTx>,
}

impl SignalHandler {
    /// Create a new signal handler. We'll have space for 2 control messages
    /// at a time, to ensure the channel isn't blocking.
    pub fn new() -> (Self, SignalRx) {
        let (tx, rx) = mpsc::channel(2);

        (
            Self {
                tx,
                shutdown_txs: vec![],
            },
            rx,
        )
    }

    /// Takes a stream who's elements are convertible to `SignalTo`, and
    /// spawns a permanent takes for transmitting to the receiver.
    pub fn forever<T, S>(&mut self, stream: S)
    where
        T: Into<SignalTo> + Send + Sync,
        S: Stream<Item = T> + 'static + Send,
    {
        let tx = self.tx.clone();

        tokio::spawn(async move {
            let mut stream = pin!(stream);

            while let Some(value) = stream.next().await {
                if tx.send(value.into()).await.is_err() {
                    error!(message = "couldn't send signal");
                    break;
                }
            }
        });
    }

    /// Takes a stream, sending to the underlying signal receiver. Returns
    /// a broadcast tx channel which can be used by the caller to either
    /// subscribe to cancellation, or trigger it. Useful for providers that
    /// may need to do both.
    pub fn add<T, S>(&mut self, stream: S)
    where
        T: Into<SignalTo> + Send,
        S: Stream<Item = T> + 'static + Send,
    {
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(2);
        let tx = self.tx.clone();

        self.shutdown_txs.push(shutdown_tx);

        tokio::spawn(async move {
            let mut stream = pin!(stream);

            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.recv() => break,
                    Some(value) = stream.next() => {
                        if tx.send(value.into()).await.is_err() {
                            error!(message = "couldn't send signal");
                            break;
                        }
                    }
                    else => {
                        error!(message = "underlying stream is closed");
                        break;
                    }
                }
            }
        });
    }

    /// Shutdown active signal handlers.
    pub fn clear(&mut self) {
        for shutdown_tx in self.shutdown_txs.drain(..) {
            // an error just means the channel was already shut down; saft to ignore
            let _ = shutdown_tx.send(());
        }
    }
}

pin_project! {
    pub struct Signals {
        #[pin]
        sigint: Signal,
        #[pin]
        sigterm: Signal,
        #[pin]
        sigquit: Signal,
        #[pin]
        sighup: Signal
    }
}

impl Stream for Signals {
    type Item = SignalTo;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.sigint.poll_recv(cx).is_ready() {
            info!(message = "Signal received", signal = "SIGINT");
            return Poll::Ready(Some(SignalTo::Shutdown));
        }

        if this.sigterm.poll_recv(cx).is_ready() {
            info!(message = "Signal received", signal = "SIGTERM");
            return Poll::Ready(Some(SignalTo::Shutdown));
        }

        if this.sigquit.poll_recv(cx).is_ready() {
            info!(message = "Signal received", signal = "SIGQUIT");
            return Poll::Ready(Some(SignalTo::Quit));
        }

        if this.sighup.poll_recv(cx).is_ready() {
            info!(message = "Signal received", signal = "SIGHUP");
            return Poll::Ready(Some(SignalTo::ReloadFromDisk));
        }

        Poll::Pending
    }
}

/// Signals from OS/user
pub fn os_signals() -> Signals {
    use tokio::signal::unix::{signal, SignalKind};

    let sigint = signal(SignalKind::interrupt()).expect("Failed to set up SIGINT handle");
    let sigterm = signal(SignalKind::terminate()).expect("Failed to set up SIGTERM handle");
    let sigquit = signal(SignalKind::quit()).expect("Failed to set up SIGQUIT handle");
    let sighup = signal(SignalKind::hangup()).expect("Failed to set up SIGHUP handle");

    Signals {
        sigint,
        sigterm,
        sigquit,
        sighup,
    }
}
