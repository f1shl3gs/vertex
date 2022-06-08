use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::metric::{MetricObserver, Observation};

#[derive(Clone, Debug, Default)]
pub struct Counter {
    pub(crate) state: Arc<AtomicU64>,
}

impl Counter {
    pub fn inc(&self, i: u64) {
        self.state.fetch_add(i, Ordering::Relaxed);
    }

    pub fn fetch(&self) -> u64 {
        self.state.load(Ordering::Relaxed)
    }
}

impl MetricObserver for Counter {
    type Recorder = Self;

    fn recorder(&self) -> Self::Recorder {
        self.clone()
    }

    fn observe(&self) -> Observation {
        Observation::Counter(self.fetch())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::default();

        assert_eq!(counter.fetch(), 0);
        counter.inc(1);
        assert_eq!(counter.fetch(), 1);
        counter.inc(2);
        assert_eq!(counter.fetch(), 3);

        counter.inc(u64::MAX);
        assert_eq!(counter.fetch(), 3);
    }
}
