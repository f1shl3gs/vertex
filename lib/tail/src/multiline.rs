use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use futures::{Stream, StreamExt};
use pin_project_lite::pin_project;
use tokio::time::{Sleep, sleep};

pub trait Logic: Clone {
    fn is_start(&mut self, line: &[u8]) -> bool;

    fn merge(&self, stashed: &mut BytesMut, data: Bytes) {
        stashed.put_u8(b'\n');
        stashed.put(data);
    }
}

pin_project! {
    pub struct Multiline<S, L, E> {
        #[pin]
        inner: S,

        logic: L,

        timeout: Duration,

        deadline: Option<Pin<Box<Sleep>>>,

        stashed: Option<(BytesMut, usize)>,

        error: Option<E>
    }
}

impl<S, L, E> Multiline<S, L, E> {
    pub fn new(inner: S, logic: L, timeout: Duration) -> Self {
        Self {
            inner,
            logic,
            timeout,
            deadline: None,
            stashed: None,
            error: None,
        }
    }

    // /// Returns a reference to the underlying stream
    // pub fn get_ref(&self) -> &S {
    //     &self.inner
    // }
}

impl<S, L, E> Stream for Multiline<S, L, E>
where
    S: Stream<Item = Result<(Bytes, usize), E>> + Unpin,
    E: Unpin,
    L: Logic,
{
    type Item = Result<(Bytes, usize), E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            match this.inner.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok((data, offset)))) => {
                    if this.logic.is_start(&data) {
                        // new start line found
                        match this.stashed.take() {
                            Some((stashed, stashed_offset)) => {
                                *this.stashed = Some((BytesMut::from(data), offset));
                                return Poll::Ready(Some(Ok((stashed.freeze(), stashed_offset))));
                            }
                            None => {
                                *this.stashed = Some((BytesMut::from(data), offset));
                            }
                        }

                        continue;
                    }

                    // non start line
                    match this.stashed {
                        // concat the new data to the stashed
                        Some((stashed, stashed_offset)) => {
                            this.logic.merge(stashed, data);
                            *stashed_offset += offset;
                        }
                        None => {
                            *this.stashed = Some((BytesMut::from(data), offset));
                        }
                    }
                }
                Poll::Ready(Some(Err(err))) => {
                    return match this.stashed.take() {
                        Some((stashed, stashed_offset)) => {
                            Poll::Ready(Some(Ok((stashed.freeze(), stashed_offset))))
                        }
                        None => Poll::Ready(Some(Err(err))),
                    };
                }
                // inner stream stopped
                Poll::Ready(None) => {
                    return match this.stashed.take() {
                        None => Poll::Ready(None),
                        Some((stashed, offset)) => {
                            Poll::Ready(Some(Ok((stashed.freeze(), offset))))
                        }
                    };
                }
                Poll::Pending => {
                    // no stashed data, so no need to wait for timeout
                    if this.stashed.is_none() {
                        return Poll::Pending;
                    }

                    // there are some data in the stashed
                    match &mut this.deadline {
                        Some(sleep) => {
                            return match sleep.as_mut().poll(cx) {
                                Poll::Ready(_) => {
                                    // timeout
                                    let Some((stashed, offset)) = this.stashed.take() else {
                                        panic!("timeout on nothing");
                                    };

                                    Poll::Ready(Some(Ok((stashed.freeze(), offset))))
                                }
                                Poll::Pending => Poll::Pending,
                            };
                        }
                        None => {
                            *this.deadline = Some(Box::pin(sleep(*this.timeout)));
                            continue;
                        }
                    };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    #[derive(Clone)]
    pub struct NoIndent;

    impl Logic for NoIndent {
        fn is_start(&mut self, line: &[u8]) -> bool {
            if let [first, ..] = line {
                !first.is_ascii_whitespace()
            } else {
                // empty line
                true
            }
        }
    }

    fn assert_output(input: Vec<Result<(Bytes, usize), ()>>, expect: Vec<(&'static str, usize)>) {
        assert_eq!(input.len(), expect.len());

        for (want, got) in expect.iter().zip(input) {
            let got = got.unwrap();

            assert_eq!(want.0, got.0);
            assert_eq!(want.1, got.1);
        }
    }

    pin_project! {
        struct LogReceiver {
            #[pin]
            rx: UnboundedReceiver<&'static str>
        }
    }

    impl Stream for LogReceiver {
        type Item = Result<(Bytes, usize), ()>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();

            match this.rx.poll_recv(cx) {
                Poll::Ready(Some(data)) => {
                    Poll::Ready(Some(Ok((Bytes::from_static(data.as_ref()), 1))))
                }
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl LogReceiver {
        fn new() -> (UnboundedSender<&'static str>, Self) {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

            (tx, LogReceiver { rx })
        }
    }

    #[tokio::test]
    async fn happy_path() {
        let inner = futures::stream::iter(vec!["foo", "  bar", "foo", "foo", "  bar"])
            .map(|line| Ok((Bytes::from(line), 1)));

        let stream = Multiline::new(inner, NoIndent, Duration::from_millis(200));

        let logs = stream.collect::<Vec<_>>().await;

        assert_output(logs, vec![("foo\n  bar", 2), ("foo", 1), ("foo\n  bar", 2)])
    }

    #[tokio::test]
    async fn non_start_line() {
        let inner = futures::stream::iter(vec!["  bar", "  bar", "foo"])
            .map(|line| Ok((Bytes::from(line), 1)));

        let stream = Multiline::new(inner, NoIndent, Duration::from_millis(200));

        let logs: Vec<Result<(Bytes, usize), ()>> = stream.collect::<Vec<_>>().await;
        // assert_eq!(logs.len(), 2);

        for result in logs {
            let (data, line) = result.unwrap();
            println!("{:02} {}", line, String::from_utf8_lossy(&data));
        }
    }

    #[tokio::test]
    async fn non_start_line_and_timeout() {
        let (tx, inner) = LogReceiver::new();
        let mut stream = Multiline::new(inner, NoIndent, Duration::from_millis(200));

        let handle = tokio::spawn(async move {
            tx.send("  bar").unwrap();
            println!("send first");
            sleep(Duration::from_millis(100)).await;

            tx.send("  bar").unwrap();
            println!("send second");
            sleep(Duration::from_secs(1)).await;

            tx.send("  bar").unwrap();
            println!("send third");
            sleep(Duration::from_secs(1)).await;

            println!("background done");
        });

        assert_eq!(
            stream.next().await,
            Some(Ok((Bytes::from_static(b"  bar\n  bar"), 2)))
        );
        assert_eq!(
            stream.next().await,
            Some(Ok((Bytes::from_static(b"  bar"), 1)))
        );

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn file_without_ending_newline() {}
}
