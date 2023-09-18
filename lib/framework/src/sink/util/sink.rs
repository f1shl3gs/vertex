use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use event::EventStatus;
use futures::{ready, FutureExt, Sink, Stream, TryFutureExt};
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use pin_project_lite::pin_project;
use tokio::sync::oneshot;
use tokio::time::{sleep, Sleep};
use tower::{Service, ServiceBuilder};
use tracing_futures::Instrument;

use super::buffer::partition::Partition;
use crate::batch::{Batch, EncodedBatch, EncodedEvent, FinalizersBatch, PushResult, StatefulBatch};
use crate::sink::util::service::{Map, ServiceBuilderExt};
use crate::sink::util::{PartitionBuffer, PartitionInnerBuffer};

pin_project! {
    /// A Partition based batcher, given some `Service` and `Batch` where the
    /// input is partitionable via the `Partition` trait, it will hold many
    /// inflight batches.
    ///
    /// This type is similar to `BatchSink` with the added benefit that it has
    /// more fine grained partitioning ability. It will hold many different
    /// batches of events and contain linger timeouts for each.
    ///
    /// Note that, unlike `BatchSink`, the `batch` given to this is *only* used
    /// to create new batches (via `Batch::fresh`) for each new partition.
    ///
    /// # Acking
    ///
    /// Service based acking will only ack events when all prior request
    /// batches have been acked. This means if sequential requests r1, r2 and
    /// r3 are dispatched and r2 and r3 complete, all events contained in
    /// all requests will not be acked until r1 has completed.
    ///
    /// # Ordering
    /// Per partition ordering can be achieved by holding onto future of a
    /// request until it finishes. Until then all further requests in that
    /// partition are delayed.
    pub struct PartitionBatchSink<S, B, K>
    where
        B: Batch,
        S: Service<B::Output>,
    {
        service: ServiceSink<S, B::Output>,
        buffer: Option<(K, EncodedEvent<B::Input>)>,
        batch: StatefulBatch<FinalizersBatch<B>>,
        partitions: HashMap<K, StatefulBatch<FinalizersBatch<B>>>,
        timeout: Duration,
        lingers: HashMap<K, Pin<Box<Sleep>>>,
        inflight: Option<HashMap<K, BoxFuture<'static, ()>>>,
        closing: bool,
    }
}

impl<S, B, K> Debug for PartitionBatchSink<S, B, K>
where
    S: Service<B::Output> + Debug,
    B: Batch + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("PartitionBatchSink")
            .field("service", &self.service)
            .field("batch", &self.batch)
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl<S, B, K> PartitionBatchSink<S, B, K>
where
    B: Batch,
    B::Input: Partition<K>,
    K: Hash + Eq + Clone + Send + 'static,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    pub fn new(service: S, batch: B, timeout: Duration) -> Self {
        Self {
            service: ServiceSink::new(service),
            buffer: None,
            batch: StatefulBatch::from(FinalizersBatch::from(batch)),
            partitions: HashMap::new(),
            timeout,
            lingers: HashMap::new(),
            inflight: None,
            closing: false,
        }
    }

    /// Enforces per partition ordering of request
    pub fn ordered(&mut self) {
        self.inflight = Some(HashMap::new())
    }
}

impl<S, B, K> Sink<EncodedEvent<B::Input>> for PartitionBatchSink<S, B, K>
where
    B: Batch,
    B::Input: Partition<K>,
    K: Hash + Eq + Clone + Send + 'static,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    type Error = crate::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.buffer.is_some() {
            match self.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => {
                    if self.buffer.is_some() {
                        return Poll::Pending;
                    }
                }
            }
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: EncodedEvent<B::Input>,
    ) -> Result<(), Self::Error> {
        let partition = item.item.partition();

        let batch = loop {
            if let Some(batch) = self.partitions.get_mut(&partition) {
                break batch;
            }

            let batch = self.batch.fresh();
            self.partitions.insert(partition.clone(), batch);

            let delay = sleep(self.timeout);
            self.lingers.insert(partition.clone(), Box::pin(delay));
        };

        if let PushResult::Overflow(item) = batch.push(item) {
            self.buffer = Some((partition, item));
        }

        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        loop {
            // Poll inner service while not ready, if we don't have buffer or any batch
            if self.buffer.is_none() && self.partitions.is_empty() {
                ready!(self.service.poll_complete(cx));
                return Poll::Ready(Ok(()));
            }

            // Try send batches
            let this = self.as_mut().project();
            let mut partitions_ready = vec![];
            for (partition, batch) in this.partitions.iter() {
                if ((*this.closing && !batch.is_empty())
                    || batch.was_full()
                    || matches!(
                        this.lingers
                            .get_mut(partition)
                            .expect("linger should exists for poll_flush")
                            .poll_unpin(cx),
                        Poll::Ready(())
                    ))
                    && this
                        .inflight
                        .as_mut()
                        .and_then(|map| map.get_mut(partition))
                        .map(|req| matches!(req.poll_unpin(cx), Poll::Ready(())))
                        .unwrap_or(true)
                {
                    partitions_ready.push(partition.clone());
                }
            }

            let mut batch_consumed = false;
            for partition in partitions_ready.iter() {
                let service_ready = match this.service.poll_ready(cx) {
                    Poll::Ready(Ok(())) => true,
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Pending => false,
                };

                if service_ready {
                    trace!(message = "Service ready; sending batch",);

                    let batch = this.partitions.remove(partition).unwrap();
                    this.lingers.remove(partition);

                    let batch = batch.finish();
                    let fut = tokio::spawn(this.service.call(batch));

                    if let Some(map) = this.inflight.as_mut() {
                        map.insert(partition.clone(), fut.map(|_| ()).fuse().boxed());
                    }

                    batch_consumed = true;
                } else {
                    break;
                }
            }

            if batch_consumed {
                continue;
            }

            // Cleanup of inflight futures
            if let Some(inflight) = this.inflight.as_mut() {
                if inflight.len() > this.partitions.len() {
                    // There is at least one in flight future without a partition to
                    // check it so we will do it here.
                    let partitions = this.partitions;
                    inflight.retain(|partition, req| {
                        partitions.contains_key(partition) || req.poll_unpin(cx).is_pending()
                    });
                }
            }

            // Try move item from buffer to batch
            if let Some((partition, item)) = self.buffer.take() {
                if self.partitions.contains_key(&partition) {
                    self.buffer = Some((partition, item));
                } else {
                    self.as_mut().start_send(item)?;

                    if self.buffer.is_some() {
                        unreachable!("Empty buffer overflowed");
                    }

                    continue;
                }
            }

            // Only poll inner service and return `Poll::Pending` anyway
            ready!(self.service.poll_complete(cx));
            return Poll::Pending;
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        trace!(message = "Closing partition batch sink");

        self.closing = true;
        self.poll_flush(cx)
    }
}

struct ServiceSink<S, R> {
    service: S,
    inflight: FuturesUnordered<oneshot::Receiver<()>>,
    seq_head: usize,
    seq_tail: usize,
    pending_acks: HashMap<usize, usize>,
    next_request_id: usize,

    _pd: PhantomData<R>,
}

impl<S, R> Debug for ServiceSink<S, R>
where
    S: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceSink")
            .field("service", &self.service)
            .field("seq_head", &self.seq_head)
            .field("seq_tail", &self.seq_tail)
            .field("pending_acks", &self.pending_acks)
            .finish()
    }
}

impl<S, R> ServiceSink<S, R>
where
    S: Service<R>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    fn new(service: S) -> Self {
        Self {
            service,
            inflight: FuturesUnordered::new(),
            seq_head: 0,
            seq_tail: 0,
            pending_acks: HashMap::new(),
            next_request_id: 0,
            _pd: PhantomData,
        }
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<crate::Result<()>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, batch: EncodedBatch<R>) -> BoxFuture<'static, ()> {
        let EncodedBatch {
            items,
            finalizers,
            count,
            byte_size,
        } = batch;
        self.seq_head += 1;

        let (tx, rx) = oneshot::channel();
        self.inflight.push(rx);
        let request_id = self.next_request_id;
        self.next_request_id = request_id.wrapping_add(1);

        trace!(
            message = "Submitting service request",
            inflight = self.inflight.len()
        );
        self.service
            .call(items)
            .err_into()
            .map(move |result| {
                let status = result_status(result);
                finalizers.update_status(status);
                if status == EventStatus::Delivered {
                    trace!(message = "Events sent", count, byte_size);
                }

                // If the rx end is dropped we still completed the request
                // so this is a weird case that we can ignore for now.
                let _ = tx.send(());
            })
            .instrument(info_span!("request", %request_id))
            .boxed()
    }

    fn poll_complete(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        while !self.inflight.is_empty() {
            match ready!(Pin::new(&mut self.inflight).poll_next(cx)) {
                Some(Ok(())) => {}
                Some(Err(_)) => panic!("ServiceSink service sender dropped."),
                None => break,
            }
        }

        Poll::Ready(())
    }
}

pub trait ServiceLogic: Clone {
    type Response: Response;

    fn result_status(&self, result: crate::Result<Self::Response>) -> EventStatus;
}

pub struct StdServiceLogic<R> {
    _pd: PhantomData<R>,
}

impl<R> Clone for StdServiceLogic<R>
where
    R: Response + Send,
{
    fn clone(&self) -> Self {
        Self { _pd: PhantomData }
    }
}

impl<R> Default for StdServiceLogic<R> {
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}

impl<R> ServiceLogic for StdServiceLogic<R>
where
    R: Response + Send,
{
    type Response = R;

    #[inline]
    fn result_status(&self, result: crate::Result<Self::Response>) -> EventStatus {
        result_status(result)
    }
}

fn result_status<R: Response + Send>(result: crate::Result<R>) -> EventStatus {
    match result {
        Ok(resp) => {
            if resp.is_successful() {
                trace!(message = "Response successful.", ?resp);
                EventStatus::Delivered
            } else if resp.is_transient() {
                error!(message = "Response wasn't successful.", ?resp);
                EventStatus::Errored
            } else {
                error!(message = "Response failed.", ?resp);
                EventStatus::Rejected
            }
        }
        Err(error) => {
            error!(message = "Request failed.", %error);
            EventStatus::Errored
        }
    }
}

// Response
pub trait Response: Debug {
    fn is_successful(&self) -> bool {
        true
    }

    fn is_transient(&self) -> bool {
        true
    }
}

impl Response for () {}

impl<'a> Response for &'a str {}

pin_project! {
    /// A `Sink` interface that wraps a `Service` and a `Batch`.
    ///
    /// Provided a batching schema, a service and batch settings
    /// this type will handle buffering events via the batching
    /// scheme and dispatching requests via the service based on
    /// either the size of the batch or a batch linger timeout.
    ///
    /// # Acking
    ///
    /// Service based acking will only ack events when all prior
    /// request batches have been acked. This means if sequential
    /// request r1, r2, and r3 are dispatched and r2 and r3 complete,
    /// all events contained in all requests will not be acked
    /// until r1 has completed.
    #[derive(Debug)]
    pub struct BatchSink<S, B>
    where
        S: Service<B::Output>,
        B: Batch,
    {
        #[pin]
        inner: PartitionBatchSink<
            Map<S, PartitionInnerBuffer<B::Output, ()>, B::Output>,
            PartitionBuffer<B, ()>,
            (),
        >,
    }
}

impl<S, B> BatchSink<S, B>
where
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    B: Batch,
{
    pub fn new(service: S, batch: B, timeout: Duration) -> Self {
        let service = ServiceBuilder::new()
            .map(|req: PartitionInnerBuffer<B::Output, ()>| req.into_parts().0)
            .service(service);
        let batch = PartitionBuffer::new(batch);
        let inner = PartitionBatchSink::new(service, batch, timeout);

        Self { inner }
    }
}

#[cfg(test)]
impl<S, B> BatchSink<S, B>
where
    B: Batch,
    S: Service<B::Output>,
{
    pub fn get_ref(&self) -> &S {
        &self.inner.service.service.inner
    }
}

impl<S, B> Sink<EncodedEvent<B::Input>> for BatchSink<S, B>
where
    B: Batch,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    type Error = crate::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: EncodedEvent<B::Input>) -> Result<(), Self::Error> {
        self.project()
            .inner
            .start_send(item.map(|item| PartitionInnerBuffer::new(item, ())))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.project().inner.poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error;
    use std::sync::atomic::AtomicUsize;
    use std::{
        convert::Infallible,
        sync::{atomic::Ordering::Relaxed, Arc, Mutex},
    };

    use bytes::Bytes;
    use event::{BatchNotifier, BatchStatus, EventFinalizer, EventFinalizers};
    use futures::{future, stream, task::noop_waker_ref, SinkExt, StreamExt};
    use tokio::{task::yield_now, time::Instant};

    use super::*;
    use crate::batch::BatchSettings;
    use crate::sink::util::buffer::vec::EncodedLength;
    use crate::sink::util::VecBuffer;

    const TIMEOUT: Duration = Duration::from_secs(10);

    type Counter = Arc<AtomicUsize>;

    impl EncodedLength for usize {
        fn encoded_length(&self) -> usize {
            22
        }
    }

    struct Request(usize, EventFinalizers);

    impl Request {
        fn new(value: usize, counter: &Counter) -> Self {
            let (batch, receiver) = BatchNotifier::new_with_receiver();
            let counter = Arc::clone(counter);

            tokio::spawn(async move {
                if receiver.await == BatchStatus::Delivered {
                    counter.fetch_add(value, Relaxed);
                }
            });

            Self(value, EventFinalizers::new(EventFinalizer::new(batch)))
        }

        fn encoded(value: usize, counter: &Counter) -> EncodedEvent<Self> {
            let mut item = Self::new(value, counter);
            let finalizers = std::mem::take(&mut item.1);

            EncodedEvent {
                item,
                finalizers,
                byte_size: 0,
            }
        }
    }

    impl EncodedLength for Request {
        fn encoded_length(&self) -> usize {
            22
        }
    }

    async fn advance_time(duration: Duration) {
        tokio::time::pause();
        tokio::time::advance(duration).await;
        tokio::time::resume();
    }

    #[tokio::test]
    async fn batch_sink_acking_sequential() {
        let ack_counter = Counter::default();

        let svc = tower::service_fn(|_| future::ok::<_, Error>(()));
        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;

        let buffered = BatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        buffered
            .sink_map_err(drop)
            .send_all(
                &mut stream::iter(0..=22).map(|item| Ok(Request::encoded(item, &ack_counter))),
            )
            .await
            .unwrap();

        assert_eq!(ack_counter.load(Relaxed), 22 * 23 / 2);
    }

    #[tokio::test]
    async fn batch_sink_acking_unordered() {
        let ack_counter = Counter::default();

        crate::trace::test_init();

        // Services future will be spawned and work between `yield_now` calls.
        let svc = tower::service_fn(|req: Vec<Request>| async move {
            let duration = match req[0].0 {
                1..=3 => Duration::from_secs(1),

                // The 4th request will introduce some sort of
                // latency spike to ensure later events don't
                // get acked.
                4 => Duration::from_secs(5),
                5 | 6 => Duration::from_secs(1),
                _ => unreachable!(),
            };

            sleep(duration).await;
            Ok::<(), Infallible>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 1;

        let mut sink = BatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let mut cx = Context::from_waker(noop_waker_ref());
        for item in 1..=3 {
            assert!(matches!(
                sink.poll_ready_unpin(&mut cx),
                Poll::Ready(Ok(()))
            ));
            assert!(matches!(
                sink.start_send_unpin(Request::encoded(item, &ack_counter)),
                Ok(())
            ));
        }

        // Clear internal buffer
        assert!(matches!(sink.poll_flush_unpin(&mut cx), Poll::Pending));
        assert_eq!(ack_counter.load(Relaxed), 0);

        yield_now().await;
        advance_time(Duration::from_secs(3)).await;
        yield_now().await;

        for _ in 1..=3 {
            assert!(matches!(
                sink.poll_flush_unpin(&mut cx),
                Poll::Ready(Ok(()))
            ));
        }

        // Events 1,2,3 should have been acked at this point.
        assert_eq!(ack_counter.load(Relaxed), 6);

        for item in 4..=6 {
            assert!(matches!(
                sink.poll_ready_unpin(&mut cx),
                Poll::Ready(Ok(()))
            ));
            assert!(matches!(
                sink.start_send_unpin(Request::encoded(item, &ack_counter)),
                Ok(())
            ));
        }

        // Clear internal buffer
        assert!(matches!(sink.poll_flush_unpin(&mut cx), Poll::Pending));
        assert_eq!(ack_counter.load(Relaxed), 6);

        yield_now().await;
        advance_time(Duration::from_secs(2)).await;
        yield_now().await;

        assert!(matches!(sink.poll_flush_unpin(&mut cx), Poll::Pending));

        // Check that events 1-3,5,6 have been acked
        assert_eq!(ack_counter.load(Relaxed), 17);

        yield_now().await;
        advance_time(Duration::from_secs(5)).await;
        yield_now().await;

        for _ in 4..=6 {
            assert!(matches!(
                sink.poll_flush_unpin(&mut cx),
                Poll::Ready(Ok(()))
            ));
        }

        assert_eq!(ack_counter.load(Relaxed), 21);
    }

    #[tokio::test]
    async fn batch_sink_buffers_messages_until_limit() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);

            sent_requests.lock().unwrap().push(req);

            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;
        let buffered = BatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        buffered
            .sink_map_err(drop)
            .send_all(&mut stream::iter(0..22).map(|item| Ok(EncodedEvent::new(item, 0))))
            .await
            .unwrap();

        let output = sent_requests.lock().unwrap();
        assert_eq!(
            &*output,
            &vec![
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
                vec![20, 21]
            ]
        );
    }

    #[tokio::test]
    async fn batch_sink_flushes_below_min_on_close() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;
        let mut buffered = BatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let mut cx = Context::from_waker(noop_waker_ref());
        assert!(matches!(
            buffered.poll_ready_unpin(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(matches!(
            buffered.start_send_unpin(EncodedEvent::new(0, 0)),
            Ok(())
        ));
        assert!(matches!(
            buffered.poll_ready_unpin(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(matches!(
            buffered.start_send_unpin(EncodedEvent::new(1, 0)),
            Ok(())
        ));

        buffered.close().await.unwrap();

        let output = sent_requests.lock().unwrap();
        assert_eq!(&*output, &vec![vec![0, 1]]);
    }

    #[tokio::test]
    async fn batch_sink_expired_linger() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;
        let mut buffered = BatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let mut cx = Context::from_waker(noop_waker_ref());
        assert!(matches!(
            buffered.poll_ready_unpin(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(matches!(
            buffered.start_send_unpin(EncodedEvent::new(0, 0)),
            Ok(())
        ));
        assert!(matches!(
            buffered.poll_ready_unpin(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(matches!(
            buffered.start_send_unpin(EncodedEvent::new(1, 0)),
            Ok(())
        ));

        // Move clock forward by linger timeout + 1 sec
        advance_time(TIMEOUT + Duration::from_secs(1)).await;

        // Flush buffer and make sure that this didn't take long time (because linger elapsed).
        let start = Instant::now();
        buffered.flush().await.unwrap();
        let elapsed = start.duration_since(start);
        assert!(elapsed < Duration::from_millis(200));

        let output = sent_requests.lock().unwrap();
        assert_eq!(&*output, &vec![vec![0, 1]]);
    }

    #[tokio::test]
    async fn partition_batch_sink_buffers_messages_until_limit() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;

        let sink = PartitionBatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        sink.sink_map_err(drop)
            .send_all(&mut stream::iter(0..22).map(|item| Ok(EncodedEvent::new(item, 0))))
            .await
            .unwrap();

        let output = sent_requests.lock().unwrap();
        assert_eq!(
            &*output,
            &vec![
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
                vec![10, 11, 12, 13, 14, 15, 16, 17, 18, 19],
                vec![20, 21]
            ]
        );
    }

    #[tokio::test]
    async fn partition_batch_sink_buffers_by_partition_buffer_size_one() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 1;

        let sink = PartitionBatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let input = vec![Partitions::A, Partitions::B];
        sink.sink_map_err(drop)
            .send_all(&mut stream::iter(input).map(|item| Ok(EncodedEvent::new(item, 0))))
            .await
            .unwrap();

        let mut output = sent_requests.lock().unwrap();
        output[..].sort();
        assert_eq!(&*output, &vec![vec![Partitions::A], vec![Partitions::B]]);
    }

    #[tokio::test]
    async fn partition_batch_sink_buffers_by_partition_buffer_size_two() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 2;

        let sink = PartitionBatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let input = vec![Partitions::A, Partitions::B, Partitions::A, Partitions::B];
        sink.sink_map_err(drop)
            .send_all(&mut stream::iter(input).map(|item| Ok(EncodedEvent::new(item, 0))))
            .await
            .unwrap();

        let mut output = sent_requests.lock().unwrap();
        output[..].sort();
        assert_eq!(
            &*output,
            &vec![
                vec![Partitions::A, Partitions::A],
                vec![Partitions::B, Partitions::B]
            ]
        );
    }

    #[tokio::test]
    async fn partition_batch_sink_submits_after_linger() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            sent_requests.lock().unwrap().push(req);
            future::ok::<_, Error>(())
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;

        let mut sink = PartitionBatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);

        let mut cx = Context::from_waker(noop_waker_ref());
        assert!(matches!(
            sink.poll_ready_unpin(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(matches!(
            sink.start_send_unpin(EncodedEvent::new(1, 0)),
            Ok(())
        ));
        assert!(matches!(sink.poll_flush_unpin(&mut cx), Poll::Pending));

        advance_time(TIMEOUT + Duration::from_secs(1)).await;

        let start = Instant::now();
        sink.flush().await.unwrap();
        let elapsed = start.duration_since(start);
        assert!(elapsed < Duration::from_millis(200));

        let output = sent_requests.lock().unwrap();
        assert_eq!(&*output, &vec![vec![1]]);
    }

    #[tokio::test]
    async fn service_sink_doesnt_propagate_error() {
        let ack_counter = Counter::default();

        // We need a mock executor here because we need to ensure
        // that we poll the service futures within the mock clock
        // context. This allows us to manually advance the time on the
        // "spawned" futures.
        let svc = tower::service_fn(|req: Request| {
            if req.0 == 3 {
                future::err("bad")
            } else {
                future::ok("good")
            }
        });
        let mut sink = ServiceSink::new(svc);
        let req = |items: usize| {
            let mut req = Request::new(items, &ack_counter);
            let finalizers = std::mem::take(&mut req.1);
            EncodedBatch {
                items: req,
                finalizers,
                count: items,
                byte_size: 1,
            }
        };

        // send some initial requests
        let mut fut1 = sink.call(req(1));
        let mut fut2 = sink.call(req(2));

        assert_eq!(ack_counter.load(Relaxed), 0);

        let mut cx = Context::from_waker(noop_waker_ref());
        assert!(matches!(fut1.poll_unpin(&mut cx), Poll::Ready(())));
        assert!(matches!(fut2.poll_unpin(&mut cx), Poll::Ready(())));
        assert!(matches!(sink.poll_complete(&mut cx), Poll::Ready(())));

        yield_now().await;
        assert_eq!(ack_counter.load(Relaxed), 3);

        // send one request that will error and one normal
        let mut fut3 = sink.call(req(3)); // I will error
        let mut fut4 = sink.call(req(4));

        // make sure they all "worked"
        assert!(matches!(fut3.poll_unpin(&mut cx), Poll::Ready(())));
        assert!(matches!(fut4.poll_unpin(&mut cx), Poll::Ready(())));
        assert!(matches!(sink.poll_complete(&mut cx), Poll::Ready(())));

        yield_now().await;
        assert_eq!(ack_counter.load(Relaxed), 7);
    }

    #[tokio::test]
    async fn partition_batch_sink_ordering_per_partition() {
        let sent_requests = Arc::new(Mutex::new(Vec::new()));

        let mut delay = true;
        let svc = tower::service_fn(|req| {
            let sent_requests = Arc::clone(&sent_requests);
            if delay {
                // Delay and then error
                delay = false;
                sleep(Duration::from_secs(1))
                    .map(move |_| {
                        sent_requests.lock().unwrap().push(req);
                        Result::<_, Error>::Ok(())
                    })
                    .boxed()
            } else {
                sent_requests.lock().unwrap().push(req);
                future::ok::<_, Error>(()).boxed()
            }
        });

        let mut batch_settings = BatchSettings::default();
        batch_settings.size.bytes = 9999;
        batch_settings.size.events = 10;

        let mut sink = PartitionBatchSink::new(svc, VecBuffer::new(batch_settings.size), TIMEOUT);
        sink.ordered();

        let input = (0..20).map(|i| (0, i)).chain((0..20).map(|i| (1, i)));
        sink.sink_map_err(drop)
            .send_all(&mut stream::iter(input).map(|item| Ok(EncodedEvent::new(item, 0))))
            .await
            .unwrap();

        let output = sent_requests.lock().unwrap();
        // We sended '0' partition first and delayed sending only first request, first 10 events,
        // which should delay sending the second batch of events in the same partition until
        // the first one succeeds.
        assert_eq!(
            &*output,
            &vec![
                (0..10).map(|i| (1, i)).collect::<Vec<_>>(),
                (10..20).map(|i| (1, i)).collect(),
                (0..10).map(|i| (0, i)).collect(),
                (10..20).map(|i| (0, i)).collect(),
            ]
        );
    }

    #[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
    enum Partitions {
        A,
        B,
    }

    impl EncodedLength for Partitions {
        fn encoded_length(&self) -> usize {
            10 // Dummy value
        }
    }

    impl Partition<Bytes> for Partitions {
        fn partition(&self) -> Bytes {
            format!("{:?}", self).into()
        }
    }

    impl Partition<Bytes> for usize {
        fn partition(&self) -> Bytes {
            "key".into()
        }
    }

    impl Partition<Bytes> for u8 {
        fn partition(&self) -> Bytes {
            "key".into()
        }
    }

    impl Partition<Bytes> for i32 {
        fn partition(&self) -> Bytes {
            "key".into()
        }
    }

    impl Partition<Bytes> for Vec<i32> {
        fn partition(&self) -> Bytes {
            "key".into()
        }
    }

    impl Partition<Bytes> for (usize, usize) {
        fn partition(&self) -> Bytes {
            self.0.to_string().into()
        }
    }

    impl EncodedLength for (usize, usize) {
        fn encoded_length(&self) -> usize {
            16
        }
    }
}
