use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::metric::{MetricObserver, Observation};

#[derive(Clone, Debug, Default)]
pub struct Gauge {
    pub(crate) state: Arc<AtomicU64>,
}

impl Gauge {
    pub fn inc(&self, value: impl Into<f64>) {
        let value = value.into();
        let mut old_u64 = self.state.load(Ordering::Relaxed);
        let mut old_f64;

        loop {
            old_f64 = f64::from_bits(old_u64);
            let new = f64::to_bits(old_f64 + value);

            match self.state.compare_exchange_weak(
                old_u64,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => old_u64 = x,
            }
        }
    }

    pub fn dec(&self, value: impl Into<f64>) {
        let value = value.into();
        let mut old_u64 = self.state.load(Ordering::Relaxed);
        let mut old_f64;

        loop {
            old_f64 = f64::from_bits(old_u64);
            let new = f64::to_bits(old_f64 - value);

            match self.state.compare_exchange_weak(
                old_u64,
                new,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => old_u64 = x,
            }
        }
    }

    pub fn set(&self, value: f64) {
        self.state.swap(f64::to_bits(value), Ordering::Relaxed);
    }

    pub fn fetch(&self) -> f64 {
        f64::from_bits(self.state.load(Ordering::Relaxed))
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

        assert_eq!(gauge.fetch(), 0.0);
        gauge.inc(2.0);
        assert_eq!(gauge.fetch(), 2.0);
        gauge.dec(1.0);
        assert_eq!(gauge.fetch(), 1.0);
        gauge.set(10.0);
        assert_eq!(gauge.fetch(), 10.0);
    }
}
