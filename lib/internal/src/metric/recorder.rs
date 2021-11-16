use std::collections::BTreeMap;
use std::sync::Arc;

use metrics::{GaugeValue, Key, Recorder, Unit};
use metrics_util::{Generational, MetricKind, MetricKindMask, Tracked, Recency};
use event::{Bucket, Metric, MetricValue};
use quanta::Clock;

use super::handle::Handle;

pub type Registry = metrics_util::Registry<Key, Handle, Tracked<Handle>>;

/// InternalRecorder is a metric recorder implementation that's suitable
/// for the advanced usage that we have in Vertex
#[derive(Clone)]
pub struct InternalRecorder {
    recency: Arc<Recency<Key>>,
    registry: Arc<Registry>,
}

impl InternalRecorder {
    pub fn new() -> Self {
        let registry = Arc::new(Registry::tracked());
        let recency = Recency::new(
            Clock::new(),
            MetricKindMask::ALL,
            Some(std::time::Duration::from_secs(5 * 60)),
        );

        Self {
            recency: Arc::new(recency),
            registry,
        }
    }

    /// Take a snapshot of all gathered metrics and expose them as metric
    pub fn capture_metrics(&self) -> impl Iterator<Item=Metric> {
        let handlers = self.registry.get_handles();
        let mut metrics = Vec::with_capacity(handlers.len());

        for ((kind, key), (gen, handle)) in handlers.into_iter() {
            if !self.recency.should_store(kind, &key, gen, &self.registry) {
                continue;
            }

            metrics.push(metric_from_kv(&key, &handle));
        }

        metrics.into_iter()
    }
}

fn metric_from_kv(key: &metrics::Key, handle: &Handle) -> Metric {
    let value = match handle {
        Handle::Counter(counter) => MetricValue::Sum(counter.count() as f64),
        Handle::Gauge(gauge) => MetricValue::Gauge(gauge.gauge()),
        Handle::Histogram(histogram) => {
            let buckets: Vec<Bucket> = histogram.buckets()
                .map(|(upper, count)| Bucket { upper, count })
                .collect();

            MetricValue::Histogram {
                count: histogram.count(),
                sum: histogram.sum(),
                buckets,
            }
        }
    };

    let tags = key.labels()
        .map(|label| (String::from(label.key()), String::from(label.value())))
        .collect::<BTreeMap<String, String>>();

    Metric {
        name: key.name().to_string(),
        description: None,
        tags,
        unit: None,
        timestamp: None,
        value,
    }
}

impl Recorder for InternalRecorder {
    fn register_counter(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
        self.registry.op(MetricKind::Counter, key, |_| {}, Handle::counter)
    }

    fn register_gauge(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
        self.registry.op(MetricKind::Gauge, key, |_| {}, Handle::gauge)
    }

    fn register_histogram(&self, key: &Key, _unit: Option<Unit>, _description: Option<&'static str>) {
        self.registry.op(MetricKind::Histogram, key, |_| {}, Handle::histogram)
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        self.registry
            .op(
                MetricKind::Counter,
                key,
                |h| h.increment_counter(value),
                Handle::counter,
            );
    }

    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        self.registry
            .op(
                MetricKind::Gauge,
                key,
                |handle| handle.update_gauge(value),
                Handle::gauge,
            );
    }

    fn record_histogram(&self, key: &Key, value: f64) {
        self.registry.op(
            MetricKind::Histogram,
            key,
            |handle| handle.record_histogram(value),
            Handle::histogram,
        )
    }
}