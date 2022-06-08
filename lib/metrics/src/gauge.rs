use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::metric::{MetricObserver, Observation};

#[derive(Clone, Debug, Default)]
pub struct Gauge {
    pub(crate) state: Arc<AtomicU64>,
}

impl Gauge {
    pub fn inc(&self, value: u64) {
        self.state.fetch_add(value, Ordering::Relaxed);
    }

    pub fn dec(&self, value: u64) {
        self.state.fetch_min(value, Ordering::Relaxed);
    }

    pub fn fetch(&self) -> u64 {
        self.state.load(Ordering::Relaxed)
    }
}

impl MetricObserver for Gauge {
    type Recorder = Self;

    fn recorder(&self) -> Self::Recorder {
        self.clone()
    }

    fn observe(&self) -> Observation {
        Observation::Gauge(self.fetch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge() {
        let gauge = Gauge::default();

        assert_eq!(gauge.fetch(), 0);
        gauge.inc(2);
        assert_eq!(gauge.fetch(), 2);
        gauge.dec(1);
        assert_eq!(gauge.fetch(), 1);
    }
}
