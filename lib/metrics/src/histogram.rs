use std::iter;
use std::iter::once;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::metric::{MakeMetricObserver, MetricObserver, Observation};

/// A bucketed observation
#[derive(Clone, Debug)]
pub struct ObservationBucket {
    pub count: u64,
    pub le: f64,
}

#[derive(Clone, Debug)]
pub struct HistogramObservation {
    pub buckets: Vec<ObservationBucket>,
    pub sum: f64,
}

#[derive(Clone, Debug)]
pub struct Histogram {
    state: Arc<Mutex<HistogramObservation>>,
}

impl Histogram {
    pub fn new(buckets: impl Iterator<Item = f64>) -> Self {
        let buckets = buckets
            .chain(once(f64::INFINITY))
            .map(|le| ObservationBucket { le, count: 0 })
            .collect::<Vec<_>>();

        Self {
            state: Arc::new(Mutex::new(HistogramObservation { buckets, sum: 0.0 })),
        }
    }

    pub fn record(&self, value: f64) {
        let mut state = self.state.lock();

        if let Some(bucket) = state.buckets.iter_mut().find(|b| value <= b.le) {
            bucket.count = bucket.count.wrapping_add(1);
            state.sum += value;
        }
    }

    pub fn get(&self) -> HistogramObservation {
        self.state.lock().clone()
    }
}

impl MetricObserver for Histogram {
    type Recorder = Self;

    fn recorder(&self) -> Self::Recorder {
        self.clone()
    }

    fn observe(&self) -> Observation {
        Observation::Histogram(self.get())
    }
}

impl MakeMetricObserver for Histogram {
    type Options = Vec<f64>;

    fn create(options: &Self::Options) -> Self {
        if options.is_empty() {
            return Histogram::new(exponential_buckets(1.0, 2.0, 10));
        };

        let mut buckets = options.clone();
        buckets.dedup();
        buckets.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Histogram::new(buckets.into_iter())
    }
}

pub fn exponential_buckets(start: f64, factor: f64, length: u64) -> impl Iterator<Item = f64> {
    iter::repeat(())
        .take(length as usize)
        .enumerate()
        .map(move |(i, _)| start * factor.powf(i as f64))
}

pub fn linear_buckets(start: f64, width: f64, length: u64) -> impl Iterator<Item = f64> {
    iter::repeat(())
        .take(length as usize)
        .enumerate()
        .map(move |(i, _)| start + (width * (i as f64)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram() {
        let buckets = vec![20.0, 40.0, 50.0];
        let histogram = Histogram::new(buckets.into_iter());

        histogram.record(10.0);

        let h = histogram.get();
        assert_eq!(h.sum, 10.0)
    }

    #[test]
    fn exponential() {
        assert_eq!(
            vec![1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0],
            exponential_buckets(1.0, 2.0, 10).collect::<Vec<_>>()
        );
    }

    #[test]
    fn linear() {
        assert_eq!(
            vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
            linear_buckets(0.0, 1.0, 10).collect::<Vec<_>>()
        );
    }
}
