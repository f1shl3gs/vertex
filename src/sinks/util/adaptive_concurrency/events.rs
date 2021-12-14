use std::time::Duration;

use internal::InternalEvent;

pub struct AdaptiveConcurrencyInflight {
    pub inflight: u64,
}

impl InternalEvent for AdaptiveConcurrencyInflight {
    fn emit_metrics(&self) {
        histogram!("adaptive_concurrency_inflight", self.inflight as f64);
    }
}

pub struct AdaptiveConcurrencyObservedRtt {
    pub rtt: Duration,
}

impl InternalEvent for AdaptiveConcurrencyObservedRtt {
    fn emit_metrics(&self) {
        histogram!("adaptive_concurrency_observed_rtt", self.rtt);
    }
}

pub struct AdaptiveConcurrencyAverageRtt {
    pub rtt: Duration,
}

impl InternalEvent for AdaptiveConcurrencyAverageRtt {
    fn emit_metrics(&self) {
        histogram!("adaptive_concurrency_averaged_rtt", self.rtt);
    }
}

pub struct AdaptiveConcurrencyLimitChanged {
    pub concurrency: u64,
    pub reached_limit: bool,
    pub had_back_pressure: bool,
    pub current_rtt: Option<Duration>,
    pub past_rtt: Duration,
    pub past_rtt_deviation: Duration,
}

impl InternalEvent for AdaptiveConcurrencyLimitChanged {
    fn emit_logs(&self) {
        trace!(
            message = "Changed concurrency",
            concurrency = %self.concurrency,
            reached_limit = %self.reached_limit,
            had_back_pressure = %self.had_back_pressure,
            current_rtt = ?self.current_rtt,
            past_rtt = ?self.past_rtt,
            past_rtt_deviation = ?self.past_rtt_deviation
        )
    }

    fn emit_metrics(&self) {
        // These are histograms, as they may have a number of different values over
        // each reporting interval, and each of those values is valuable for diagnosis.
        histogram!("adaptive_concurrency_limit", self.concurrency as f64);
        let reached_limit = self.reached_limit.then(|| 1.0).unwrap_or_default();
        histogram!("adaptive_concurrency_reached_limit", reached_limit);
        let back_pressure = self.had_back_pressure.then(|| 1.0).unwrap_or_default();
        histogram!("adaptive_concurrency_back_pressure", back_pressure);
        histogram!("adaptive_concurrency_past_rtt_mean", self.past_rtt);
    }
}
