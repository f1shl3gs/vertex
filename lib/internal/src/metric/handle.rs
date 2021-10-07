use metrics::GaugeValue;
use std::{
    slice,
    sync::Arc,
    sync::atomic::{AtomicU32, AtomicU64, Ordering},
};

#[derive(Debug)]
struct AtomicF64 {
    inner: AtomicU64,
}

impl AtomicF64 {
    fn new(init: f64) -> Self {
        Self {
            inner: AtomicU64::new(init.to_bits())
        }
    }

    fn fetch_update<F>(
        &self,
        set_order: Ordering,
        fetch_order: Ordering,
        mut f: F,
    ) -> Result<f64, f64>
        where
            F: FnMut(f64) -> Option<f64>,
    {
        let res = self.inner.fetch_update(set_order, fetch_order, |x| {
            let opt: Option<f64> = f(f64::from_bits(x));
            opt.map(|i| i.to_bits())
        });

        res.map(f64::from_bits).map_err(f64::from_bits)
    }

    fn load(&self, order: Ordering) -> f64 {
        f64::from_bits(self.inner.load(order))
    }
}

#[derive(Clone, Debug)]
pub enum Handle {
    Gauge(Arc<Gauge>),
    Counter(Arc<Counter>),
    Histogram(Arc<Histogram>),
}

impl Handle {
    pub fn counter() -> Self {
        Handle::Counter(Arc::new(Counter::new()))
    }

    pub fn increment_counter(&self, value: u64) {
        match self {
            Handle::Counter(counter) => counter.record(value),
            _ => unreachable!()
        }
    }

    pub fn gauge() -> Self {
        Handle::Gauge(Arc::new(Gauge::new()))
    }

    pub fn update_gauge(&self, value: GaugeValue) {
        match self {
            Handle::Gauge(gauge) => gauge.record(value),
            _ => unreachable!()
        }
    }

    pub fn histogram() -> Self {
        Handle::Histogram(Arc::new(Histogram::new()))
    }

    pub fn record_histogram(&self, value: f64) {
        match self {
            Handle::Histogram(h) => h.record(value),
            _ => unreachable!()
        }
    }
}

#[derive(Debug)]
pub struct Gauge {
    inner: AtomicF64,
}

impl Gauge {
    pub fn new() -> Self {
        Self {
            inner: AtomicF64::new(0.0)
        }
    }

    pub fn record(&self, value: GaugeValue) {
        // Because Rust lacks an atomic f64 we store gauges as AtomicU64
        // and transmute back and forth to an f64 here. They have the
        // same size so this operation is safe, just don't read the
        // AtomicU64 directly
        self.inner
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |c| {
                let v = value.update_value(c);
                Some(v)
            });
    }

    pub fn gauge(&self) -> f64 {
        self.inner.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct Histogram {
    buckets: Box<[(f64, AtomicU32); 22]>,
    count: AtomicU64,
    sum: AtomicF64,
}

impl Histogram {
    pub fn new() -> Self {
        // Box to avoid having this large array inline to the structure,
        // blowing out cache coherence.
        //
        // The sequence here is based on powers of two. Other sequences are more
        // suitable for different distributions but since our present use case
        // is mostly non-negative and measures smallish latencies we cluster
        // around but never quite get to zero with an increasingly coarse long-tail
        let buckets = Box::new([
            (f64::NEG_INFINITY, AtomicU32::new(0)),
            (0.015_625, AtomicU32::new(0)),
            (0.03125, AtomicU32::new(0)),
            (0.0625, AtomicU32::new(0)),
            (0.125, AtomicU32::new(0)),
            (0.25, AtomicU32::new(0)),
            (0.5, AtomicU32::new(0)),
            (0.0, AtomicU32::new(0)),
            (1.0, AtomicU32::new(0)),
            (2.0, AtomicU32::new(0)),
            (4.0, AtomicU32::new(0)),
            (8.0, AtomicU32::new(0)),
            (16.0, AtomicU32::new(0)),
            (32.0, AtomicU32::new(0)),
            (64.0, AtomicU32::new(0)),
            (128.0, AtomicU32::new(0)),
            (256.0, AtomicU32::new(0)),
            (512.0, AtomicU32::new(0)),
            (1024.0, AtomicU32::new(0)),
            (2048.0, AtomicU32::new(0)),
            (4096.0, AtomicU32::new(0)),
            (f64::INFINITY, AtomicU32::new(0)),
        ]);

        Self {
            buckets,
            count: AtomicU64::new(0),
            sum: AtomicF64::new(0.0),
        }
    }

    pub fn record(&self, value: f64) {
        let mut prev_bound = f64::NEG_INFINITY;
        for (bound, bucket) in self.buckets.iter() {
            if value > prev_bound && value <= *bound {
                bucket.fetch_add(1, Ordering::Relaxed);
                break;
            }

            prev_bound = *bound;
        }

        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum
            .fetch_update(Ordering::AcqRel, Ordering::Relaxed, |c| Some(c + value));
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn sum(&self) -> f64 {
        self.sum.load(Ordering::Relaxed)
    }

    pub fn buckets(&self) -> BucketIter<'_> {
        BucketIter {
            inner: self.buckets.iter()
        }
    }
}

pub struct BucketIter<'a> {
    inner: slice::Iter<'a, (f64, AtomicU32)>,
}

impl<'a> Iterator for BucketIter<'a> {
    type Item = (f64, u32);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (*k, v.load(Ordering::Relaxed)))
    }
}

#[derive(Debug)]
pub struct Counter {
    inner: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            inner: AtomicU64::new(0)
        }
    }

    pub fn with_count(count: u64) -> Self {
        Self {
            inner: AtomicU64::new(count)
        }
    }

    pub fn record(&self, value: u64) {
        self.inner
            .fetch_add(value, Ordering::Relaxed);
    }

    pub fn count(&self) -> u64 {
        self.inner.load(Ordering::Relaxed)
    }
}

