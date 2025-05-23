use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use futures::{Stream, StreamExt, ready};
use pin_project_lite::pin_project;
use tokio::time::interval;

use crate::stats;

pin_project! {
    pub struct Utilization<S> {
        timer: Timer,
        intervals: tokio::time::Interval,
        inner: S,
    }
}

impl<S> Utilization<S> {
    /// Consumes this wrapper and returns the inner stream.
    ///
    /// This can't be constant because destructors can't be run in a const context, and we're
    /// discarding `Interval`/`Timer` when we call this.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S> Stream for Utilization<S>
where
    S: Stream + Unpin,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // The goal of this function is to measure the time between when the
        // caller requests the next Event from the stream and before one is
        // ready, with the side-effect of reporting every so often about how
        // long the wait gap is.
        //
        // To achieve this we poll the `intervals` stream and if a new interval
        // is ready we hit `Timer::report` and loop back around again to poll
        // for a new `Event`. Calls to `Timer::start_wait` will only have an
        // effect if `stop_wait` has been called, so the structure of this loop
        // avoids double-measures.
        let this = self.project();
        loop {
            this.timer.start_wait();
            match this.intervals.poll_tick(cx) {
                Poll::Ready(_) => {
                    this.timer.report();
                    continue;
                }
                Poll::Pending => {
                    let result = ready!(this.inner.poll_next_unpin(cx));
                    this.timer.stop_wait();
                    return Poll::Ready(result);
                }
            }
        }
    }
}

/// Wrap a stream to emit stats about utilization. This is designed for use with
/// the input channels of transform and sinks components, and measures the
/// amount of time that the stream is waiting for input from upstream. We make
/// the simplifying assumption that this wait time is when the component is idle
/// and the rest of the time it is doing useful work. This is more true for
/// sinks than transforms, which can be blocked by downstream components, but
/// with knowledge of the config the data is still useful.
pub fn wrap<S>(inner: S) -> Utilization<S> {
    Utilization {
        timer: Timer::new(),
        intervals: interval(Duration::from_secs(5)),
        inner,
    }
}

pub struct Timer {
    overall_start: Instant,
    span_start: Instant,
    waiting: bool,
    total_wait: Duration,
    ewma: stats::Ewma,

    // metrics
    utilization: metrics::Gauge,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            overall_start: Instant::now(),
            span_start: Instant::now(),
            waiting: false,
            total_wait: Duration::new(0, 0),
            ewma: stats::Ewma::new(0.9),
            utilization: metrics::register_gauge("utilization", "A ratio from 0 to 1 of the load on a component. A value of 0 would indicate a completely idle component that is simply waiting for input. A value of 1 would indicate a that is never idle. This value is updated every 5 seconds.").recorder(&[]),
        }
    }
}

/// A simple, specialized timer for tracking spans of waiting vs not-waiting
/// time and reporting a smoothed estimate of utilization.
///
/// This implementation uses the idea of spans and reporting periods. Spans are
/// a period of time spent entirely in one state, aligning with state
/// transitions but potentially more granular.  Reporting periods are expected
/// to be of uniform length and used to aggregate span data into time-weighted
/// averages.
impl Timer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a new span representing time spent waiting
    pub fn start_wait(&mut self) {
        if !self.waiting {
            self.end_span();
            self.waiting = true;
        }
    }

    /// Complete the current waiting span and begin a non-waiting span
    pub fn stop_wait(&mut self) -> Instant {
        if self.waiting {
            let now = self.end_span();
            self.waiting = false;
            now
        } else {
            Instant::now()
        }
    }

    /// Meant to be called on a regular interval, this method calculates wait
    /// ratio since the last time it was called and reports the resulting
    /// utilization average.
    pub fn report(&mut self) {
        // End the current span so it can be accounted for, but do not change
        // whether or not we're in the waiting state. This way the next span
        // inherits the correct status.
        let now = self.end_span();

        let total_duration = now.duration_since(self.overall_start);
        let wait_ratio = self.total_wait.as_secs_f64() / total_duration.as_secs_f64();
        let utilization = 1.0 - wait_ratio;

        self.ewma.update(utilization);
        let avg = self.ewma.average().unwrap_or(f64::NAN);
        debug!(utilization = %avg);
        self.utilization.set(avg);

        // Reset overall statistics for the next reporting period.
        self.overall_start = self.span_start;
        self.total_wait = Duration::new(0, 0);
    }

    fn end_span(&mut self) -> Instant {
        if self.waiting {
            self.total_wait += self.span_start.elapsed();
        }
        self.span_start = Instant::now();
        self.span_start
    }
}
