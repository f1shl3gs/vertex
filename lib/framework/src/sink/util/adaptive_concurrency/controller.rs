use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

#[cfg(test)]
use testify::stats::{TimeHistogram, TimeWeightedSum};
use tokio::sync::OwnedSemaphorePermit;
use tokio::time::error::Elapsed;

use crate::http::HttpError;
use crate::stats::{EwmaVar, Mean, MeanVariance};

use super::semaphore::ShrinkableSemaphore;
use super::AdaptiveConcurrencySettings;
use super::MAX_CONCURRENCY;
use crate::sink::util::retries::{RetryAction, RetryLogic};

/// Shared class for `tokio::sync::Semaphore` that manages adjusting the
/// semaphore size and other associated data.
#[derive(Clone, Debug)]
pub struct Controller<L> {
    semaphore: Arc<ShrinkableSemaphore>,
    concurrency: Option<usize>,
    settings: AdaptiveConcurrencySettings,
    logic: L,
    pub inner: Arc<Mutex<Inner>>,

    #[cfg(test)]
    pub stats: Arc<Mutex<ControllerStatistics>>,

    // metrics
    limit: metrics::Histogram,
    reached_limit: metrics::Histogram,
    back_pressure: metrics::Histogram,
    past_rtt_mean: metrics::Histogram,
    averaged_rtt: metrics::Histogram,
    observed_rtt: metrics::Histogram,
    inflight: metrics::Histogram,
}

#[derive(Debug)]
pub struct Inner {
    pub(crate) current_limit: usize,
    inflight: usize,
    past_rtt: EwmaVar,
    next_update: Instant,
    current_rtt: Mean,
    had_back_pressure: bool,
    reached_limit: bool,
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct ControllerStatistics {
    pub inflight: TimeHistogram,
    pub concurrency_limit: TimeHistogram,
    pub observed_rtt: TimeWeightedSum,
    pub averaged_rtt: TimeWeightedSum,
}

impl<L> Controller<L> {
    pub fn new(
        concurrency: Option<usize>,
        settings: AdaptiveConcurrencySettings,
        logic: L,
    ) -> Self {
        // If a `concurrency` is specified, it becomes both the current limit and the maximum,
        // effectively bypassing all the mechanisms. Otherwise, the current limit is set to 1
        // and the maximum to MAX_CONCURRENCY.
        let current_limit = concurrency.unwrap_or(1);

        let limit = metrics::register_histogram(
            "adaptive_concurrency_limit",
            "The concurrency limit that the adaptive concurrency feature has decided on for this current window.",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let reached_limit = metrics::register_histogram(
            "adaptive_concurrency_reached_limit",
            "",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let back_pressure = metrics::register_histogram(
            "adaptive_concurrency_back_pressure",
            "",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let past_rtt_mean = metrics::register_histogram(
            "adaptive_concurrency_past_rtt_mean",
            "",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let averaged_rtt = metrics::register_histogram(
            "adaptive_concurrency_averaged_rtt",
            "The average round-trip time (RTT) for the current window.",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let observed_rtt = metrics::register_histogram(
            "adaptive_concurrency_observed_rtt_seconds",
            "The observed round-trip time (RTT) for requests.",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);
        let inflight = metrics::register_histogram(
            "adaptive_concurrency_inflight",
            "The number of outbound requests currently awaiting a response.",
            metrics::exponential_buckets(1.0, 2.0, 10),
        )
        .recorder(&[]);

        Self {
            semaphore: Arc::new(ShrinkableSemaphore::new(current_limit)),
            concurrency,
            settings,
            logic,
            inner: Arc::new(Mutex::new(Inner {
                current_limit,
                inflight: 0,
                past_rtt: EwmaVar::new(settings.ewma_alpha),
                next_update: instant_now(),
                current_rtt: Default::default(),
                had_back_pressure: false,
                reached_limit: false,
            })),
            #[cfg(test)]
            stats: Arc::new(Mutex::new(Default::default())),

            // metrics
            limit,
            reached_limit,
            back_pressure,
            past_rtt_mean,
            averaged_rtt,
            observed_rtt,
            inflight,
        }
    }

    pub fn acquire(&self) -> impl Future<Output = OwnedSemaphorePermit> + Send + 'static {
        Arc::clone(&self.semaphore).acquire()
    }

    pub fn start_request(&self) {
        let mut inner = self.inner.lock().expect("Controller mutex is poisoned");

        #[cfg(test)]
        {
            let mut stats = self.stats.lock().expect("Stats mutex is poisoned");
            stats.inflight.add(inner.inflight, instant_now());
        }

        inner.inflight += 1;
        if inner.inflight >= inner.current_limit {
            inner.reached_limit = true;
        }

        self.inflight.record(inner.inflight as f64);
    }

    /// Adjust the controller to a response, based on type of response given (backpresuree or not)
    /// and if it should be used as a valid RTT measurement.
    fn adjust_to_response_inner(&self, start: Instant, is_back_pressure: bool, use_rtt: bool) {
        let now = instant_now();
        let mut inner = self.inner.lock().expect("Controller mutex is poisoned");

        let rtt = now.saturating_duration_since(start);
        if use_rtt {
            self.observed_rtt.record(rtt.as_secs_f64());
        }

        let rtt = rtt.as_secs_f64();
        if is_back_pressure {
            inner.had_back_pressure = true;
        }

        #[cfg(test)]
        let mut stats = self.stats.lock().expect("Stats mutex is poisoned");

        #[cfg(test)]
        {
            if use_rtt {
                stats.observed_rtt.add(rtt, now);
            }

            stats.inflight.add(inner.inflight, now);
        }

        inner.inflight -= 1;
        self.inflight.record(inner.inflight as f64);

        if use_rtt {
            inner.current_rtt.update(rtt);
        }
        let current_rtt = inner.current_rtt.average();

        // When the RTT values are all exactly the same, as for the "constant link" test,
        // the average calculation above produces results either the exact value or that value
        // plus epsilon, depending on the number of samples. This ends up throttling aggressively
        // due to the high side falling outside of the calculated deviance. Rounding these values
        // forces the differences to zero.
        #[cfg(test)]
        let current_rtt = current_rtt.map(|c| (c * 1000000.0).round() / 1000000.0);

        match inner.past_rtt.state() {
            None => {
                // No past measurements, set up initial values.
                if let Some(current_rtt) = current_rtt {
                    inner.past_rtt.update(current_rtt);
                    inner.next_update = now + Duration::from_secs_f64(current_rtt);
                }
            }

            Some(mut past_rtt) => {
                if now < inner.next_update {
                    return;
                }

                #[cfg(test)]
                {
                    if let Some(current_rtt) = current_rtt {
                        stats.averaged_rtt.add(current_rtt, now);
                    }

                    stats.concurrency_limit.add(inner.current_limit, now);
                    drop(stats); // Drop the stats lock a little earlier on this path
                }

                if let Some(current_rtt) = current_rtt {
                    self.averaged_rtt.record(current_rtt);
                }

                // Only manage the concurrency if `concurrency` was set to "adaptive"
                if self.concurrency.is_none() {
                    self.manage_limit(&mut inner, past_rtt, current_rtt);
                }

                // Reset values for next interval
                if let Some(current_rtt) = current_rtt {
                    past_rtt = inner.past_rtt.update(current_rtt);
                }
                inner.next_update = now + Duration::from_secs_f64(past_rtt.mean);
                inner.current_rtt = Default::default();
                inner.had_back_pressure = false;
                inner.reached_limit = false;
            }
        }
    }

    fn manage_limit(
        &self,
        inner: &mut MutexGuard<Inner>,
        past_rtt: MeanVariance,
        current_rtt: Option<f64>,
    ) {
        let past_rtt_deviation = past_rtt.variance.sqrt();
        let threshold = past_rtt_deviation * self.settings.rtt_deviation_scale;

        // Normal quick responses trigger an increase in the concurrency limit. Note that we only
        // check this if we had requests to go beyond the current limit ot prevent increasing the
        // limit beyond what we have evidence for.
        if inner.current_limit < MAX_CONCURRENCY
            && inner.reached_limit
            && !inner.had_back_pressure
            && current_rtt.is_some()
            && current_rtt.unwrap() <= past_rtt.mean
        {
            // Increase(additive) the current concurrency limit
            self.semaphore.add_permits(1);
            inner.current_limit += 1;
        } else if inner.current_limit > 1
            && (inner.had_back_pressure || current_rtt.unwrap_or(0.0) >= past_rtt.mean + threshold)
        {
            // Back pressure responses, either explicit or implicit due to increasing response times,
            // trigger a decrease in the concurrency limit.

            // Decrease(multiplicative) the current concurrency limit
            let to_forget = inner.current_limit
                - (inner.current_limit as f64 * self.settings.decrease_ratio) as usize;
            self.semaphore.forget_permits(to_forget);
            inner.current_limit -= to_forget;
        }

        self.limit.record(inner.current_limit as f64);
        let reached_limit = inner.reached_limit.then_some(1.0).unwrap_or_default();
        self.reached_limit.record(reached_limit);
        let back_pressure = inner.had_back_pressure.then_some(1.0).unwrap_or_default();
        self.back_pressure.record(back_pressure);
        self.past_rtt_mean.record(past_rtt.mean);

        trace!(
            message = "Changed concurrency",
            concurrency = %inner.current_limit as u64,
            reached_limit = %reached_limit,
            had_back_pressure = %back_pressure,
            current_rtt = ?current_rtt.map(Duration::from_secs_f64),
            past_rtt = ?Duration::from_secs_f64(past_rtt.mean),
            past_rtt_deviation = ?Duration::from_secs_f64(past_rtt_deviation)
        )
    }
}

impl<L> Controller<L>
where
    L: RetryLogic,
{
    pub fn adjust_to_response(&self, start: Instant, resp: &Result<L::Response, crate::Error>) {
        // It would be better to avoid generating the string in Retry(_) just to throw it away
        // here, but it's probably not worth the effort.
        let response_action = resp.as_ref().map(|resp| self.logic.should_retry_resp(resp));
        let is_back_pressure = match &response_action {
            Ok(action) => matches!(action, RetryAction::Retry(_)),
            Err(err) => {
                if let Some(err) = err.downcast_ref::<L::Error>() {
                    self.logic.is_retriable_error(err)
                } else if err.downcast_ref::<Elapsed>().is_some() {
                    true
                } else if err.downcast_ref::<HttpError>().is_some() {
                    // HTTP protocal-level errors are not backpressure
                    false
                } else {
                    warn!(
                        message = "Unhandled error response",
                        %err,
                        internal_log_rate_limit = true
                    );

                    false
                }
            }
        };

        // Only adjust to the RTT when the request was successfully processed.
        let use_rtt = matches!(response_action, Ok(RetryAction::Successful));
        self.adjust_to_response_inner(start, is_back_pressure, use_rtt)
    }
}

pub fn instant_now() -> Instant {
    tokio::time::Instant::now().into()
}
