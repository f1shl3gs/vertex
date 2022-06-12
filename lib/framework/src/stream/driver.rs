use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::fmt;
use std::num::NonZeroUsize;
use std::task::Poll;

use buffers::{Ackable, Acker};
use event::{EventStatus, Finalizable};
use futures::{poll, FutureExt, Stream, StreamExt, TryFutureExt};
use futures_util::future::poll_fn;
use tokio::{pin, select};
use tower::Service;
use tracing::Instrument;

use super::futures_unordered_chunked::FuturesUnorderedChunked;

/// Newtype wrapper around sequence numbers to enforce misuse resistance.
#[derive(Debug, Eq, Ord, PartialOrd, PartialEq)]
struct SequenceNumber(u64);

impl SequenceNumber {
    /// Gets the actual integer value of this sequence number
    ///
    /// This can be used trivially for correlating a given `SequenceNumber`
    /// in logs/metrics/tracings
    const fn id(&self) -> u64 {
        self.0
    }
}

/// An out-of-order acknowledgement waiting to become valid
struct PendingAcknowledgement {
    seq_num: SequenceNumber,
    ack_size: usize,
}

impl PartialEq for PendingAcknowledgement {
    fn eq(&self, other: &Self) -> bool {
        self.seq_num == other.seq_num
    }
}

impl Eq for PendingAcknowledgement {}

impl PartialOrd for PendingAcknowledgement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse ordering so that in a `BinaryHeap`, the lowest sequence number
        // is the highest priority.
        Some(other.seq_num.cmp(&self.seq_num))
    }
}

impl Ord for PendingAcknowledgement {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .partial_cmp(self)
            .expect("PendingAcknowledgement should always return a valid comparison")
    }
}

#[derive(Default)]
struct AcknowledgementTracker {
    out_of_order: BinaryHeap<PendingAcknowledgement>,
    seq_head: u64,
    seq_tail: u64,
    ack_depth: usize,
}

impl AcknowledgementTracker {
    /// Acquires the next available sequence number
    fn get_next_seq_num(&mut self) -> SequenceNumber {
        let seq_num = self.seq_head;
        self.seq_head += 1;
        SequenceNumber(seq_num)
    }

    /// Marks the given sequence number as complete
    fn mark_seq_num_complete(&mut self, seq_num: SequenceNumber, ack_size: usize) {
        if seq_num.0 == self.seq_tail {
            self.ack_depth += ack_size;
            self.seq_tail += 1;
        } else {
            self.out_of_order
                .push(PendingAcknowledgement { seq_num, ack_size })
        }
    }

    /// Consumes the current acknowledgement "depth"
    ///
    /// When a sequence number is marked as complete, we either update our tail pointer
    /// if the acknowledgement is "in order" -- essentially, it was the very next sequence
    /// number we expected to see -- or store it for later if it's out-of-order
    ///
    /// In this method, we see if any of the out-of-order sequence numbers can now be
    /// applied: may be 9 sequence numbers were marked complete, but one number that
    /// came before all of them was still pending, so they had to be stored in the
    /// out-of-order list to be checked later. This is where we check them.
    ///
    /// For any sequence number -- whether it completed in order or had to be applied fr.m
    /// the out-of-order list -- there is an associated acknowledge "depth", which can be
    /// though of the amount of items the sequence is acknowledgement as complete.
    ///
    /// We accumulate that amount for every sequence number between calls to `consume_ack_depth`.
    /// Thus, a fresh instance of `AcknowledgementTracker` has an acknowledgement depth of 0. If
    /// we create five sequence numbers, and mark them all complete with an acknowledge meant of
    /// 10. our depth would then be 50. Calling this method would return `Some(50)`, and if this
    /// method was called again immediately after, it would return `None`.
    fn consume_ack_depth(&mut self) -> Option<NonZeroUsize> {
        // Drain any out-of-order acknowledgements that can now be ordered correctly.
        while let Some(ack) = self.out_of_order.peek() {
            if ack.seq_num.0 == self.seq_tail {
                let PendingAcknowledgement { ack_size, .. } = self
                    .out_of_order
                    .pop()
                    .expect("should not be here unless self.out_of_order is non-empty");

                self.ack_depth += ack_size;
                self.seq_tail += 1;
            } else {
                break;
            }
        }

        match self.ack_depth {
            0 => None,
            n => {
                self.ack_depth = 0;
                NonZeroUsize::new(n)
            }
        }
    }
}

pub trait DriverResponse {
    fn event_status(&self) -> EventStatus;
    fn events_send(&self) -> EventsSent;
}

/// Drives the interaction between a stream of items and a service which processes
/// them asynchronously.
///
/// `Driver`, as a high-level, facilitates taking items from an arbitrary `Stream`
/// and pushing them through a `Service`, spawning each call to the service so that
/// work can be run concurrently, managing waiting for the service to be ready before
/// processing more items, and so on.
///
/// Additionally, `Driver` handles two event-specific facilities: finalization and
/// acknowledgement.
///
/// This capability is parameterized so any implementation which can define how to
/// interpret the response for each request, as well as define how many events a
/// request is compromised of, can be used with `Driver`.
pub struct Driver<I, S> {
    input: I,
    service: S,
    acker: Acker,
}

impl<I, S> Driver<I, S> {
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(input: I, service: S, acker: Acker) -> Self {
        Self {
            input,
            service,
            acker,
        }
    }
}

impl<I, S> Driver<I, S>
where
    I: Stream,
    I::Item: Ackable + Finalizable,
    S: Service<I::Item>,
    S::Error: fmt::Debug + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse,
{
    /// Runs the driver until the input stream is exhausted.
    ///
    /// All in-flight calls to the provided `service` will also be completed before
    /// `run` returns.
    ///
    /// # Errors
    ///
    /// The return type is mostly to simplify caller code.
    /// An error is currently only returned if a service returns an error from `poll_ready`
    pub async fn run(self) -> Result<(), ()> {
        let mut inflight = FuturesUnorderedChunked::new(1024);
        let mut ack_tracker = AcknowledgementTracker::default();
        let mut next_batch: Option<VecDeque<I::Item>> = None;

        let Self {
            input,
            mut service,
            acker,
        } = self;
        let batched_input = input.ready_chunks(1024);
        pin!(batched_input);

        loop {
            // Core behavior of this loop
            // - always check to see if we have any response futures that have completed, if
            //   so, handling acking as many events as we can (ordering matters)
            // - if we have a "current" batch, try to send each request in it to the service
            //   if we can't drain all requests from the batch due to lack of service readiness,
            //   then put the batch back and try to send the rest of it when the service is ready
            //   again
            // - if we have no "current" batch, but there is an available batch from our input
            //   stream, grab that batch and store it as our current batch
            //
            // Essentially, we bounce back and forth between "grab the new batch from the input
            // stream" and "send all requests in the batch to our service" which could be
            // trivially modeled with a normal imperative loop. However, we want to be able to
            // interleave the acknowledgement of responses to allow buffers and sources to
            // continue making forward progress, which necessitates a more complex weaving of
            // logic. Using `select!` is more code, and requires a more careful eye than blindly
            // doing "get_next_batch().await; process_batch().await", but it does make doing the
            // complex logic easier than if we tried to interleave it ourselves with an
            // imperative-style loop.

            select! {
                // Using `biased` ensures we check the branches in the order they're written, since
                // the default behavior of the `select!` macro is to randomly order branches as a
                // means of ensuring scheduling fairness.
                biased;

                // One or more of our service calls have completed
                Some(acks) = inflight.next(), if !inflight.is_empty() => {
                    for ack in acks {
                        let (seq, ack_size): (SequenceNumber, usize) = ack;
                        let request_id = seq.id();
                        trace!(
                            message = "Acknowledging service request.",
                            request_id,
                            ack_size,
                        );

                        ack_tracker.mark_seq_num_complete(seq, ack_size);
                    }

                    if let Some(ack_depth) = ack_tracker.consume_ack_depth() {
                        trace!(
                            message = "Acking events",
                            ack_size = ack_depth,
                        );

                        acker.ack(ack_depth.get());
                    }
                }

                // We've got an input batch to process and the service is ready to accept a request.
                maybe_ready = poll_fn(|cx| service.poll_ready(cx)), if next_batch.is_some() => {
                    let mut batch = next_batch.take()
                        .expect("batch should be populated");

                    let mut maybe_ready = Some(maybe_ready);
                    while !batch.is_empty() {
                        // Make sure the service is ready to take another request.
                        let maybe_ready = match maybe_ready.take() {
                            Some(ready) => Poll::Ready(ready),
                            None => poll!(poll_fn(|cx| service.poll_ready(cx))),
                        };

                        let svc = match maybe_ready {
                            Poll::Ready(Ok(())) => &mut service,
                            Poll::Ready(Err(err)) => {
                                error!(
                                    message = "Service return error from `poll_ready()`",
                                    ?err
                                );

                                return Err(())
                            },
                            Poll::Pending => {
                                next_batch = Some(batch);
                                break
                            },
                        };

                        let mut req = batch.pop_front()
                            .expect("batch should not be empty");
                        let seq_num = ack_tracker.get_next_seq_num();
                        let request_id = seq_num.id();

                        trace!(
                            message = "Submitting service request",
                            inflight = inflight.len(),
                            request_id,
                        );

                        let ack_size = req.ack_size();
                        let finalizers = req.take_finalizers();

                        let fut = svc.call(req)
                            .err_into()
                            .map(move |result: Result<S::Response, S::Error>| {
                                match result {
                                    Err(err) => {
                                        error!(
                                            message = "Service call failed",
                                            ?err,
                                            request_id,
                                        );
                                        finalizers.update_status(EventStatus::Failed);
                                    },
                                    Ok(resp) => {
                                        trace!(
                                            message = "Service call succeeded",
                                            request_id,
                                        );

                                        finalizers.update_status(resp.event_status());
                                        internal::emit(&resp.events_send());
                                    }
                                };

                                (seq_num, ack_size)
                            })
                            .instrument(info_span!("request", request_id));

                        inflight.push(fut);
                    }
                }

                // We've received some items from the input stream.
                Some(reqs) = batched_input.next(), if next_batch.is_none() => {
                    let reqs = reqs;
                    next_batch = Some(reqs.into());
                }

                else => break
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::EventFinalizers;
    use futures::ready;
    use proptest::prelude::Strategy;
    use proptest::{prop_assert_eq, proptest};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use rand_distr::Distribution;
    use rand_distr::Pareto;
    use std::future::Future;
    use std::iter::repeat_with;
    use std::pin::Pin;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tokio::sync::{OwnedSemaphorePermit, Semaphore};
    use tokio::time::sleep;
    use tokio_util::sync::PollSemaphore;

    struct DelayRequest(usize);

    impl Ackable for DelayRequest {
        fn ack_size(&self) -> usize {
            self.0
        }
    }

    impl Finalizable for DelayRequest {
        fn take_finalizers(&mut self) -> EventFinalizers {
            EventFinalizers::default()
        }
    }

    struct DelayResponse;

    impl DriverResponse for DelayResponse {
        fn event_status(&self) -> EventStatus {
            EventStatus::Delivered
        }

        fn events_send(&self) -> EventsSent {
            EventsSent {
                count: 1,
                byte_size: 1,
                output: None,
            }
        }
    }

    impl AsRef<EventStatus> for DelayResponse {
        fn as_ref(&self) -> &EventStatus {
            &EventStatus::Delivered
        }
    }

    // Generic service that takes a usize and applies an arbitrary delay to returning it.
    struct DelayService {
        semaphore: PollSemaphore,
        permit: Option<OwnedSemaphorePermit>,
        jitter: Pareto<f64>,
        jitter_gen: StdRng,
        lower_bound_us: u64,
        upper_bound_us: u64,
    }

    impl DelayService {
        pub fn new(permits: usize, lower_bound: Duration, upper_bound: Duration) -> Self {
            assert!(upper_bound > lower_bound);

            Self {
                semaphore: PollSemaphore::new(Arc::new(Semaphore::new(permits))),
                permit: None,
                jitter: Pareto::new(1.0, 1.0).expect("distribution should be valid"),
                jitter_gen: StdRng::from_seed([
                    3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4, 6, 2, 6, 4, 3, 3,
                    8, 3, 2, 7, 9, 5,
                ]),
                lower_bound_us: lower_bound.as_micros().max(10_000) as u64,
                upper_bound_us: upper_bound.as_micros().max(10_000) as u64,
            }
        }

        pub fn get_sleep_dur(&mut self) -> Duration {
            let lower = self.lower_bound_us;
            let upper = self.upper_bound_us;

            // Generate a value between 10ms and 500ms, with a long tail shape to the distribution.
            self.jitter
                .sample_iter(&mut self.jitter_gen)
                .map(|n| n * lower as f64)
                .map(|n| n as u64)
                .filter(|n| *n > lower && *n < upper)
                .map(Duration::from_micros)
                .next()
                .expect("jitter iter should be endless")
        }
    }

    impl Service<DelayRequest> for DelayService {
        type Response = DelayResponse;
        type Error = ();
        type Future =
            Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            if self.permit.is_some() {
                panic!("should not call poll_ready again after a successful call");
            }

            match ready!(self.semaphore.poll_acquire(cx)) {
                None => panic!("semaphore should not be closed"),
                Some(permit) => assert!(self.permit.replace(permit).is_none()),
            }

            Poll::Ready(Ok(()))
        }

        fn call(&mut self, _req: DelayRequest) -> Self::Future {
            let permit = self
                .permit
                .take()
                .expect("calling `call` without successful `poll_ready` is invalid");
            let sleep_dur = self.get_sleep_dur();

            Box::pin(async move {
                sleep(sleep_dur).await;

                // Manually drop our permit here so that we take ownership and then actually
                // release the slot back to the semaphore.
                drop(permit);

                Ok(DelayResponse)
            })
        }
    }

    fn arb_shuffled_seq_num<S>(selector: S) -> impl Strategy<Value = Vec<u64>>
    where
        S: Strategy<Value = usize>,
    {
        selector
            .prop_map(|len| (0..len).into_iter().map(|n| n as u64).collect())
            .no_shrink()
            .prop_shuffle()
    }

    #[test]
    fn acknowledgement_tracker_simple() {
        let mut ack_tracker = AcknowledgementTracker::default();

        assert_eq!(ack_tracker.consume_ack_depth(), None);

        let seq1 = ack_tracker.get_next_seq_num();
        ack_tracker.mark_seq_num_complete(seq1, 42);

        assert_eq!(ack_tracker.consume_ack_depth(), NonZeroUsize::new(42));
        assert_eq!(ack_tracker.consume_ack_depth(), None);

        let seq2 = ack_tracker.get_next_seq_num();
        let seq3 = ack_tracker.get_next_seq_num();
        ack_tracker.mark_seq_num_complete(seq3, 314);
        assert_eq!(ack_tracker.consume_ack_depth(), None);

        ack_tracker.mark_seq_num_complete(seq2, 86);
        assert_eq!(ack_tracker.consume_ack_depth(), NonZeroUsize::new(400));
    }

    proptest! {
        // This test occasionally hangs,. Ignore until it can be looked at more
        #[test]
        #[ignore]
        fn acknowledgement_tracker_gauntlet(
            seq_ack_order in arb_shuffled_seq_num(0..1000usize),
            batch_size_seed in proptest::collection::vec(0..100usize, 5..=10),
            max_batch_size in 2..=10usize,
        ) {
            // `AcknowledgementTacker` uses a newtype wrapper, `SeqNumber`, to dole out its
            // sequence numbers in a way that ensures callers can't arbitrarily pass in sequence
            // numbers that out outside of the valid numbers, or numbers we've already seen.
            //
            // This makes it harder to test since we want the order of sequence number
            // acknowledgments to be driven by `proptest` itself. Thus, we take a simple but
            // slightly ugly approach: generate the raw numbers as part of the test inputs, and
            // then transmute them by generating sequence numbers for each raw input number, and
            // do a one-by-one replacement.
            //
            // We know that generated sequence numbers will always start in order, and start
            // from zero. We can also grab the internal u64 that represents a sequence number.
            // With that, we can find each integer in `seq_ack_order` for each `SequenceNumber`
            // we generate, and do a simple check at the end to make sure we've successfully
            // mapped each one.
            let mut ack_trakcer = AcknowledgementTracker::default();
            let mut total_ack_depth = 0;
            let expected_total_ack_depth: usize = seq_ack_order.iter().sum::<u64>()
                .try_into()
                .expect("total ack depth should not exceed usize");
            let mut seq_nums = (0..seq_ack_order.len())
                .map(|_| ack_trakcer.get_next_seq_num())
                .collect::<Vec<_>>();
            let mut reordered_seq_nums = seq_ack_order.iter()
                .filter_map(|n| seq_nums.iter().position(|n2| n2.id() == *n)
                .map(|i| seq_nums.swap_remove(i)))
                .collect::<VecDeque<_>>();

            assert!(seq_nums.is_empty());
            assert_eq!(seq_ack_order.len(), reordered_seq_nums.len());

            // Generate our batch sizes. We want to ensure that we're able to eventually drain
            // all sequence numbers from the input, while still letting `proptest` drive the batch
            // sizes used. This is problematic because `proptest` has no way to ask for an infinite
            // iterator natively. We approximate this by having it give us a set of batch size
            // "seeds", as well as a variable max batch size, which we use to construct our own
            // infinite iterator. this iterator is obviously not _directly_ shrinkable by
            // `proptest`, because values will immediately diverge from the seeds rather than
            // simply being cycled endlessly, but it should suffice for generating random values
            // over time that are, essentially, deterministic based on the given seed.
            let mut next_base = batch_size_seed.into_iter()
                .cycle();
            let mut last_output: usize = 0;
            let mut batch_sizes = repeat_with(move || {
                let base = next_base.next()
                    .expect("repeat iterator should never be empty");
                let modified = base + last_output;
                let next_output = modified % max_batch_size;
                last_output = next_output;
                next_output
            });

            // Now start acknowledging sequence numbers. We do this in variable-sized chunks,
            // based on `ack_batch_size`, and get the ack depth at the end of the every batch,
            // accumulating it as part of the total.
            while !reordered_seq_nums.is_empty() {
                let batch_size = batch_sizes.next()
                    .expect("repeat iterator should never be empty");
                for _ in 0..batch_size {
                    match reordered_seq_nums.pop_front() {
                        None => break,
                        Some(seq_num) => {
                            let ack_size = seq_num.id()
                                .try_into()
                                .expect("seq_num should not exceed usize");
                            ack_trakcer.mark_seq_num_complete(seq_num, ack_size);
                        },
                    }
                }

                if let Some(ack_depth) = ack_trakcer.consume_ack_depth() {
                    total_ack_depth += ack_depth.get();
                }
            }

            prop_assert_eq!(expected_total_ack_depth, total_ack_depth);
        }
    }

    #[tokio::test]
    async fn driver_simple() {
        // This test uses a server which creates response futures that sleep for a variable, but
        // bounded, amout of time, giving the impression of work being completed. Completion of
        // all requests/responses is asserted by checking that the counter used by the acker
        // matches the expected ack amount. The delays themselves are deterministic based on a
        // fixed-seed RNG, so the test should always run in a fairly constant time between runs.
        //
        // TODO: Given the use of a deterministic RNG, we could likely transition this test to be
        //   driven via `proptest`, to also allow driving the input requests. The main thing that
        //   we do not control is the arrival of requests in the input stream itself, which means
        //   that the generated batches will almost always be the biggest possible size, since
        //   the stream is always immediately available.
        //
        // It might be possible to spawn a background task to drive a true MPSC channel with
        // requests based on input provided from `proptest` to control not only the value(which
        // determines ack size) but the delay between messages, as well... simulating delays
        // between bursts of messages, similar to real sources.

        // Set up our driver input stream, service, etc
        let input_requests = (0..2048).into_iter().collect::<Vec<_>>();
        let input_total: usize = input_requests.iter().sum();
        let input_stream = futures::stream::iter(input_requests.into_iter().map(DelayRequest));
        let service = DelayService::new(10, Duration::from_millis(5), Duration::from_millis(150));
        let (acker, counter) = Acker::basic();
        let driver = Driver::new(input_stream, service, acker);

        // Now actually run the driver, consuming all of the input
        match driver.run().await {
            Ok(()) => assert_eq!(input_total, counter.load(Ordering::SeqCst)),
            Err(()) => panic!("driver unexpectedly returned with error"),
        }
    }
}
