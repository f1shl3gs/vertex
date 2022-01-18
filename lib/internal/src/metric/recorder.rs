use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use event::{Bucket, Metric};
use metrics::{Key, KeyName, Recorder, Unit};
use metrics_util::recency::{GenerationalPrimitives, Recency};
use metrics_util::{Histogram, MetricKindMask};
use parking_lot::RwLock;
use quanta::Clock;

pub type Registry = metrics_util::Registry<GenerationalPrimitives>;

#[inline]
pub fn default_histogram_bounds() -> &'static [f64] {
    &[
        f64::NEG_INFINITY,
        0.015_625,
        0.03125,
        0.0625,
        0.125,
        0.25,
        0.5,
        0.0,
        1.0,
        2.0,
        4.0,
        8.0,
        16.0,
        32.0,
        64.0,
        128.0,
        256.0,
        512.0,
        1024.0,
        2048.0,
        4096.0,
        f64::INFINITY,
    ]
}

/// InternalRecorder is a metric recorder implementation that's suitable
/// for the advanced usage that we have in Vertex
pub struct InternalRecorder {
    recency: Arc<Recency>,
    registry: Arc<Registry>,
    descriptions: RwLock<HashMap<String, &'static str>>,
    histograms: RwLock<HashMap<Key, Histogram>>,
}

impl InternalRecorder {
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());
        let recency = Recency::new(
            Clock::new(),
            MetricKindMask::ALL,
            Some(std::time::Duration::from_secs(5 * 60)),
        );

        Self {
            recency: Arc::new(recency),
            registry,
            descriptions: RwLock::new(HashMap::new()),
            histograms: Default::default(),
        }
    }

    /// Take a snapshot of all gathered metrics and expose them as metric
    pub fn capture_metrics(&self) -> impl Iterator<Item = Metric> {
        let mut metrics = vec![];

        // Counters
        let handles = self.registry.get_counter_handles();
        for (key, counter) in handles {
            let gen = counter.get_generation();
            if !self.recency.should_store_counter(&key, gen, &self.registry) {
                continue;
            }

            let (name, tags) = key_to_parts(&key);
            let value = counter.get_inner().load(Ordering::Acquire);

            metrics.push(Metric::sum_with_tags(name, "", value, tags));
        }

        // Gauges
        let handles = self.registry.get_gauge_handles();
        for (key, gauge) in handles {
            let gen = gauge.get_generation();
            if !self.recency.should_store_gauge(&key, gen, &self.registry) {
                continue;
            }

            let (name, tags) = key_to_parts(&key);
            let value = f64::from_bits(gauge.get_inner().load(Ordering::Acquire));

            metrics.push(Metric::gauge_with_tags(name, "", value, tags));
        }

        // Histograms
        let handles = self.registry.get_histogram_handles();
        for (key, histogram) in handles {
            let gen = histogram.get_generation();
            if !self
                .recency
                .should_store_histogram(&key, gen, &self.registry)
            {
                // Since we store aggregated histograms directly, when we're told that a
                // metric is not recent enough and should be/was deleted from the registry,
                // we also need to delete it on our side as well.
                let mut wg = self.histograms.write();
                wg.remove(&key);

                continue;
            }

            let mut wg = self.histograms.write();
            let entry = wg
                .entry(key)
                .or_insert_with(|| Histogram::new(default_histogram_bounds()).unwrap());

            histogram
                .get_inner()
                .clear_with(|sample| entry.record_many(sample));
        }

        for (key, histogram) in self.histograms.read().clone() {
            let (name, tags) = key_to_parts(&key);
            let buckets = histogram
                .buckets()
                .into_iter()
                .map(|(upper, count)| Bucket { upper, count })
                .collect::<Vec<_>>();

            metrics.push(Metric::histogram_with_tags(
                name,
                "",
                tags,
                histogram.count(),
                histogram.sum(),
                buckets,
            ))
        }

        metrics.into_iter()
    }

    fn add_description_if_missing(&self, name: KeyName, description: &'static str) {
        let sanitized = sanitize_metric_name(name.as_str());
        let mut descriptions = self.descriptions.write();
        descriptions.entry(sanitized).or_insert(description);
    }
}

fn key_to_parts(key: &Key) -> (String, BTreeMap<String, String>) {
    let name = key.name();
    let tags = key
        .labels()
        .map(|label| (String::from(label.key()), String::from(label.value())))
        .collect::<BTreeMap<String, String>>();

    return (name.to_string(), tags);
}

#[inline]
fn invalid_metric_name_start_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z_:].
    !(c.is_ascii_alphabetic() || c == '_' || c == ':')
}

#[inline]
fn invalid_metric_name_character(c: char) -> bool {
    // Essentially, needs to match the regex pattern of [a-zA-Z0-9_:].
    !(c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

pub fn sanitize_metric_name(name: &str) -> String {
    // The first character must be [a-zA-Z_:], and all subsequent characters must be [a-zA-Z0-9_:].
    name.replacen(invalid_metric_name_start_character, "_", 1)
        .replace(invalid_metric_name_character, "_")
}

impl Recorder for InternalRecorder {
    fn describe_counter(&self, key: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.add_description_if_missing(key, description);
    }

    fn describe_gauge(&self, key: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.add_description_if_missing(key, description);
    }

    fn describe_histogram(&self, key: KeyName, _unit: Option<Unit>, description: &'static str) {
        self.add_description_if_missing(key, description);
    }

    fn register_counter(&self, key: &Key) -> metrics::Counter {
        self.registry
            .get_or_create_counter(key, |c| c.clone().into())
    }

    fn register_gauge(&self, key: &Key) -> metrics::Gauge {
        self.registry.get_or_create_gauge(key, |c| c.clone().into())
    }

    fn register_histogram(&self, key: &Key) -> metrics::Histogram {
        self.registry
            .get_or_create_histogram(key, |c| c.clone().into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compile() {}
}
