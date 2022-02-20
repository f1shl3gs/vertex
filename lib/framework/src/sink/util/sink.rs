use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use buffers::Acker;
use event::EventStatus;
use futures::{ready, FutureExt, Sink, Stream, TryFutureExt};
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use internal::EventsSent;
use pin_project::pin_project;
use tokio::sync::oneshot;
use tokio::time::{sleep, Sleep};
use tower::{Service, ServiceBuilder};
use tracing_futures::Instrument;

use crate::batch::{Batch, EncodedBatch, EncodedEvent, FinalizersBatch, PushResult, StatefulBatch};
use crate::sink::util::partition::{Partition, PartitionBuffer, PartitionInnerBuffer};
use crate::sink::util::service::{Map, ServiceBuilderExt};

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
#[pin_project]
pub struct PartitionBatchSink<S, B, K, L>
where
    B: Batch,
    S: Service<B::Output>,
{
    service: ServiceSink<S, B::Output, L>,
    buffer: Option<(K, EncodedEvent<B::Input>)>,
    batch: StatefulBatch<FinalizersBatch<B>>,
    partitions: HashMap<K, StatefulBatch<FinalizersBatch<B>>>,
    timeout: Duration,
    lingers: HashMap<K, Pin<Box<Sleep>>>,
    inflight: Option<HashMap<K, BoxFuture<'static, ()>>>,
    closing: bool,
}

impl<S, B, K, SL> Debug for PartitionBatchSink<S, B, K, SL>
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

impl<S, B, K> PartitionBatchSink<S, B, K, StdServiceLogic<S::Response>>
where
    B: Batch,
    B::Input: Partition<K>,
    K: Hash + Eq + Clone + Send + 'static,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    pub fn new(service: S, batch: B, timeout: Duration, acker: Acker) -> Self {
        Self::new_with_logic(service, batch, timeout, acker, StdServiceLogic::default())
    }
}

impl<S, B, K, L> PartitionBatchSink<S, B, K, L>
where
    B: Batch,
    B::Input: Partition<K>,
    K: Hash + Eq + Clone + Send + 'static,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    L: ServiceLogic<Response = S::Response> + Send + 'static,
{
    pub fn new_with_logic(service: S, batch: B, timeout: Duration, acker: Acker, logic: L) -> Self {
        let service = ServiceSink::new_with_logic(service, acker, logic);

        Self {
            service,
            buffer: None,
            batch: StatefulBatch::from(FinalizersBatch::from(batch)),
            partitions: HashMap::new(),
            timeout,
            lingers: HashMap::new(),
            inflight: None,
            closing: false,
        }
    }

    /// Enforces per partition ordering of request.
    pub fn ordered(&mut self) {
        self.inflight = Some(HashMap::new())
    }
}

impl<S, B, K, L> Sink<EncodedEvent<B::Input>> for PartitionBatchSink<S, B, K, L>
where
    B: Batch,
    B::Input: Partition<K>,
    K: Hash + Eq + Clone + Send + 'static,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    L: ServiceLogic<Response = S::Response> + Send + 'static,
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

                    let batch_size = batch.num_items();
                    let batch = batch.finish();
                    let fut = tokio::spawn(this.service.call(batch, batch_size));

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

struct ServiceSink<S, R, L> {
    service: S,
    inflight: FuturesUnordered<oneshot::Receiver<(usize, usize)>>,
    acker: Acker,
    seq_head: usize,
    seq_tail: usize,
    pending_acks: HashMap<usize, usize>,
    next_request_id: usize,
    logic: L,

    _pd: PhantomData<R>,
}

impl<S, R, SL> Debug for ServiceSink<S, R, SL>
where
    S: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceSink")
            .field("service", &self.service)
            .field("acker", &self.acker)
            .field("seq_head", &self.seq_head)
            .field("seq_tail", &self.seq_tail)
            .field("pending_acks", &self.pending_acks)
            .finish()
    }
}

impl<S, R> ServiceSink<S, R, StdServiceLogic<S::Response>>
where
    S: Service<R>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
{
    #[cfg(test)]
    fn new(service: S, acker: Acker) -> Self {
        Self::new_with_logic(service, acker, StdServiceLogic::default())
    }
}

impl<S, R, L> ServiceSink<S, R, L>
where
    S: Service<R>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    L: ServiceLogic<Response = S::Response> + Send + 'static,
{
    fn new_with_logic(service: S, acker: Acker, logic: L) -> Self {
        Self {
            service,
            inflight: FuturesUnordered::new(),
            acker,
            seq_head: 0,
            seq_tail: 0,
            pending_acks: HashMap::new(),
            next_request_id: 0,
            logic,
            _pd: PhantomData,
        }
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<crate::Result<()>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, batch: EncodedBatch<R>, batch_size: usize) -> BoxFuture<'static, ()> {
        let EncodedBatch {
            items,
            finalizers,
            count,
            byte_size,
        } = batch;
        let seqno = self.seq_head;
        self.seq_head += 1;

        let (tx, rx) = oneshot::channel();
        self.inflight.push(rx);
        let request_id = self.next_request_id;
        self.next_request_id = request_id.wrapping_add(1);

        trace!(
            message = "Submitting service request",
            inflight = self.inflight.len()
        );
        let logic = self.logic.clone();
        self.service
            .call(items)
            .err_into()
            .map(move |result| {
                let status = logic.result_status(result);
                finalizers.update_status(status);
                if status == EventStatus::Delivered {
                    emit!(&EventsSent {
                        count,
                        byte_size,
                        output: None
                    });
                }

                // If the rx end is dropped we still completed the request
                // so this is a weird case that we can ignore for now.
                let _ = tx.send((seqno, batch_size));
            })
            .instrument(info_span!("request", %request_id))
            .boxed()
    }

    fn poll_complete(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        while !self.inflight.is_empty() {
            match ready!(Pin::new(&mut self.inflight).poll_next(cx)) {
                Some(Ok((seqno, batch_size))) => {
                    self.pending_acks.insert(seqno, batch_size);

                    let mut num_to_ack = 0;
                    while let Some(ack_size) = self.pending_acks.remove(&self.seq_tail) {
                        num_to_ack += ack_size;
                        self.seq_tail += 1
                    }

                    trace!(message = "Acking events", acking_num = num_to_ack);
                    self.acker.ack(num_to_ack);
                }

                Some(Err(_)) => panic!("ServiceSink service sender dropped"),

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

    fn result_status(&self, result: crate::Result<Self::Response>) -> EventStatus {
        match result {
            Ok(resp) => {
                if resp.is_successful() {
                    trace!(message = "Response successful", ?resp);

                    EventStatus::Delivered
                } else if resp.is_transient() {
                    error!(message = "Response wasn't successful", ?resp);

                    EventStatus::Errored
                } else {
                    error!(message = "Response failed", ?resp);

                    EventStatus::Failed
                }
            }
            Err(err) => {
                error!(message = "Request failed", %err);
                EventStatus::Errored
            }
        }
    }
}

// Response
pub trait Response: fmt::Debug {
    fn is_successful(&self) -> bool {
        true
    }

    fn is_transient(&self) -> bool {
        true
    }
}

impl Response for () {}

impl<'a> Response for &'a str {}

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
#[pin_project]
#[derive(Debug)]
pub struct BatchSink<S, B, L>
where
    S: Service<B::Output>,
    B: Batch,
{
    #[pin]
    inner: PartitionBatchSink<
        Map<S, PartitionInnerBuffer<B::Output, ()>, B::Output>,
        PartitionBuffer<B, ()>,
        (),
        L,
    >,
}

impl<S, B> BatchSink<S, B, StdServiceLogic<S::Response>>
where
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    B: Batch,
{
    pub fn new(service: S, batch: B, timeout: Duration, acker: Acker) -> Self {
        Self::new_with_logic(service, batch, timeout, acker, StdServiceLogic::default())
    }
}

impl<S, B, SL> BatchSink<S, B, SL>
where
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    B: Batch,
    SL: ServiceLogic<Response = S::Response> + Send + 'static,
{
    pub fn new_with_logic(
        service: S,
        batch: B,
        timeout: Duration,
        acker: Acker,
        logic: SL,
    ) -> Self {
        let service = ServiceBuilder::new()
            .map(|req: PartitionInnerBuffer<B::Output, ()>| req.into_parts().0)
            .service(service);

        let batch = PartitionBuffer::new(batch);
        let inner = PartitionBatchSink::new_with_logic(service, batch, timeout, acker, logic);
        Self { inner }
    }
}

#[cfg(test)]
impl<S, B, L> BatchSink<S, B, L>
where
    B: Batch,
    S: Service<B::Output>,
{
    pub fn get_ref(&self) -> &S {
        &self.inner.service.service.inner
    }
}

impl<S, B, SL> Sink<EncodedEvent<B::Input>> for BatchSink<S, B, SL>
where
    B: Batch,
    S: Service<B::Output>,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send + 'static,
    S::Response: Response + Send + 'static,
    SL: ServiceLogic<Response = S::Response> + Send + 'static,
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
