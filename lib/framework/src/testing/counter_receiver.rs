use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};

use event::Event;
use futures::FutureExt;
use futures::{Stream, StreamExt, TryStreamExt, ready};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tokio_util::codec::{FramedRead, LinesCodec};

pub struct CountReceiver<T> {
    count: Arc<AtomicUsize>,
    trigger: Option<oneshot::Sender<()>>,
    connected: Option<oneshot::Receiver<()>>,
    handle: JoinHandle<Vec<T>>,
}

impl<T: Send + 'static> CountReceiver<T> {
    pub fn count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Succeeds once first connection has been made.
    pub async fn connected(&mut self) {
        if let Some(tripwire) = self.connected.take() {
            tripwire.await.unwrap();
        }
    }

    fn new<F, Fut>(make_fut: F) -> CountReceiver<T>
    where
        F: FnOnce(Arc<AtomicUsize>, oneshot::Receiver<()>, oneshot::Sender<()>) -> Fut,
        Fut: Future<Output = Vec<T>> + Send + 'static,
    {
        let count = Arc::new(AtomicUsize::new(0));
        let (trigger, tripwire) = oneshot::channel();
        let (trigger_connected, connected) = oneshot::channel();

        CountReceiver {
            count: Arc::clone(&count),
            trigger: Some(trigger),
            connected: Some(connected),
            handle: tokio::spawn(make_fut(count, tripwire, trigger_connected)),
        }
    }
}

impl<T> Future for CountReceiver<T> {
    type Output = Vec<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Some(trigger) = this.trigger.take() {
            let _ = trigger.send(());
        }

        let result = ready!(this.handle.poll_unpin(cx));
        Poll::Ready(result.unwrap())
    }
}

impl CountReceiver<String> {
    pub fn receive_lines(addr: SocketAddr) -> CountReceiver<String> {
        CountReceiver::new(|count, tripwire, connected| async move {
            let listener = TcpListener::bind(addr).await.unwrap();
            CountReceiver::receive_lines_stream(
                TcpListenerStream::new(listener),
                count,
                tripwire,
                Some(connected),
            )
            .await
        })
    }

    #[cfg(unix)]
    pub fn receive_lines_unix<P>(path: P) -> CountReceiver<String>
    where
        P: AsRef<Path> + Send + 'static,
    {
        CountReceiver::new(|count, tripwire, connected| async move {
            let listener = tokio::net::UnixListener::bind(path).unwrap();
            CountReceiver::receive_lines_stream(
                UnixListenerStream::new(listener),
                count,
                tripwire,
                Some(connected),
            )
            .await
        })
    }

    async fn receive_lines_stream<S, T>(
        stream: S,
        count: Arc<AtomicUsize>,
        tripwire: oneshot::Receiver<()>,
        mut connected: Option<oneshot::Sender<()>>,
    ) -> Vec<String>
    where
        S: Stream<Item = tokio::io::Result<T>>,
        T: AsyncWrite + AsyncRead,
    {
        stream
            .take_until(tripwire)
            .map_ok(|socket| FramedRead::new(socket, LinesCodec::new()))
            .map(|x| {
                connected.take().map(|trigger| trigger.send(()));
                x.unwrap()
            })
            .flatten()
            .map(|x| x.unwrap())
            .inspect(move |_| {
                count.fetch_add(1, Ordering::Relaxed);
            })
            .collect::<Vec<String>>()
            .await
    }
}

impl CountReceiver<Event> {
    pub fn receive_events<S>(stream: S) -> CountReceiver<Event>
    where
        S: Stream<Item = Event> + Send + 'static,
    {
        CountReceiver::new(|count, tripwire, connected| async move {
            connected.send(()).unwrap();
            stream
                .take_until(tripwire)
                .inspect(move |_| {
                    count.fetch_add(1, Ordering::Relaxed);
                })
                .collect::<Vec<Event>>()
                .await
        })
    }
}
