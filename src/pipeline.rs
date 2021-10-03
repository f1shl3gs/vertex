use crate::{
    transforms::FunctionTransform,
};
use event::Event;
use std::{
    collections::VecDeque,
    fmt,
    pin::Pin,
    task::{Context, Poll},
};
use futures::channel::mpsc;

#[derive(Debug)]
pub struct ClosedError;

impl fmt::Display for ClosedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Pipeline is closed.")
    }
}

impl std::error::Error for ClosedError {}

const MAX_ENQUEUED: usize = 1024;

#[derive(Clone)]
pub struct Pipeline {
    inner: mpsc::Sender<Event>,
    enqueued: VecDeque<Event>,

    inlines: Vec<Box<dyn FunctionTransform>>,
    outstanding: usize,
}

impl Pipeline {
    #[cfg(test)]
    pub fn new_test() -> (Self, mpsc::Receiver<Event>) {
        Self::new_with_buffer(100, vec![])
    }

    fn try_flush(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), <Self as futures::Sink<Event>>::Error>> {
        // We batch the updates to "events out" for efficiency, and do it
        // here because it gives us a chance to allow the natural batching
        // of `Pipeline` to kick in
        if self.outstanding > 0 {
            self.outstanding = 0;
        }

        while let Some(event) = self.enqueued.pop_front() {
            match self.inner.poll_ready(cx) {
                Poll::Pending => {
                    self.enqueued.push_front(event);
                    return Poll::Pending;
                }

                Poll::Ready(Ok(())) => {
                    // continue to send blow
                }

                Poll::Ready(Err(_err)) => {
                    return Poll::Ready(Err(ClosedError));
                }
            }

            match self.inner.start_send(event) {
                Ok(()) => {
                    // we good, keep looping
                }

                Err(_) => {
                    return Poll::Ready(Err(ClosedError));
                }

                // Tokio's channel doesn't have those features
                //
                // Err(err) if err.is_full() => {
                //     // We only try to send after a successful call to poll_ready,
                //     // which reserves space for us in the channel. That makes this
                //     // branch unreachable as long as the channel implementation fulfills
                //     // its own contract.
                //     panic!("Channel was both ready and full; this is a bug")
                // }
//
                // Err(err) if err.is_disconnected() => {
                //     return Poll::Ready(Err(ClosedError));
                // }
//
                // Err(_) => unreachable!()
            }
        }

        Poll::Ready(Ok(()))
    }

    pub fn from_sender(
        inner: mpsc::Sender<Event>,
        inlines: Vec<Box<dyn FunctionTransform>>,
    ) -> Self {
        Self {
            inner,
            inlines,
            // We ensure the buffer is sufficient that it is unlikely to
            // require re-allocations. There is a possibility a component
            // might blow this queue size.
            enqueued: VecDeque::with_capacity(16),
            outstanding: 0,
        }
    }

    pub fn new_with_buffer(n: usize, inlines: Vec<Box<dyn FunctionTransform>>) -> (Self, mpsc::Receiver<Event>) {
        let (tx, rx) = mpsc::channel(n);
        (Self::from_sender(tx, inlines), rx)
    }
}

impl futures::Sink<Event> for Pipeline {
    type Error = ClosedError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.enqueued.len() < MAX_ENQUEUED {
            Poll::Ready(Ok(()))
        } else {
            self.try_flush(cx)
        }
    }

    fn start_send(mut self: Pin<&mut Self>, item: Event) -> Result<(), Self::Error> {
        self.outstanding += 1;

        // Note how this gets **swapped** with `new_working_set` in the loop.
        // At the end of the loop, it will only contain finalized events.
        let mut working_set = vec![item];
        for inline in self.inlines.iter_mut() {
            let mut new_working_set = Vec::with_capacity(working_set.len());
            for event in working_set.drain(..) {
                inline.transform(&mut new_working_set, event);
            }

            core::mem::swap(&mut new_working_set, &mut working_set);
        }
        self.enqueued.extend(working_set);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.try_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;
    use crate::{event::Metric};
    use crate::event::MetricValue;
    use futures::{SinkExt, StreamExt};
    use futures::task::noop_waker_ref;
    use tokio::time::{
        sleep, timeout
    };

    #[derive(Clone)]
    struct AddTag {
        k: String,
        v: String,
    }

    impl FunctionTransform for AddTag {
        fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
            let metric = event.as_mut_metric();

            metric.tags.insert(self.k.clone(), self.v.clone());

            output.push(event);
        }
    }

    async fn collect_ready<S>(mut rx: S) -> Vec<S::Item>
        where S: futures::Stream + Unpin
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

    #[tokio::test]
    async fn normal_send_recv() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);

        tx.send(true).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(true, received);
    }

    #[tokio::test]
    async fn test_send_and_recv() {
        let total = 10u64;
        let (mut tx, mut rx) = Pipeline::new_test();
        tokio::spawn(async move {
            for i in 0..total {
                let s = format!("{}", i);
                let ev = Event::from(s);
                tx.send(ev).await.unwrap();
            }
        });

        sleep(Duration::from_millis(100)).await;
        let es = timeout(Duration::from_secs(1), rx.collect::<Vec<Event>>()).await.unwrap();
        assert_eq!(es.len(), 10);
    }

    #[tokio::test]
    async fn multiple_transforms() -> Result<(), crate::Error> {
        let t1 = AddTag {
            k: "k1".into(),
            v: "k2".into(),
        };

        let t2 = AddTag {
            k: "k2".into(),
            v: "v2".into(),
        };

        let (mut pipeline, mut receiver) = Pipeline::new_with_buffer(100, vec![Box::new(t1), Box::new(t2)]);

        let event = Event::Metric(Metric {
            name: "foo".to_string(),
            description: None,
            tags: Default::default(),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge(0.1),
        });


        let closed = pipeline.inner.is_closed();
        println!("closed: {}", closed);

        pipeline.send(event).await.unwrap();

        let closed = pipeline.inner.is_closed();
        println!("closed: {}", closed);

        let _out = collect_ready(receiver).await;

        let closed = pipeline.inner.is_closed();
        println!("closed: {}, received: {}", closed, _out.len());

        Ok(())
    }
}