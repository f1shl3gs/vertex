use std::task::Poll;

use futures::task::{Context, noop_waker_ref};
use futures::{Stream, StreamExt};

pub async fn collect_ready<S>(mut rx: S) -> Vec<S::Item>
where
    S: Stream + Unpin,
{
    let waker = noop_waker_ref();
    let mut cx = Context::from_waker(waker);

    let mut vec = Vec::new();
    loop {
        match rx.poll_next_unpin(&mut cx) {
            Poll::Ready(Some(item)) => vec.push(item),
            Poll::Ready(None) | Poll::Pending => return vec,
        }
    }
}

pub async fn collect_n<S>(rx: S, n: usize) -> Vec<S::Item>
where
    S: Stream + Unpin,
{
    rx.take(n).collect().await
}

pub async fn collect_one<S>(mut rx: S) -> S::Item
where
    S: Stream + Unpin,
{
    rx.next().await.unwrap()
}
