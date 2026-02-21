use std::collections::HashMap;

use chrono::{DateTime, Utc};
use event::{EventMetadata, Metric, MetricSeries, MetricValue};

use crate::batch::{
    Batch, BatchConfig, BatchError, BatchSize, Merged, PushResult, SinkBatchSettings,
};

/// points into a batch
///
/// Batching mostly means that we will aggregate away timestamp information,
/// and apply metric-specific compression to improve the performance of the
/// pipeline. In particular, only the latest in a series of metrics are
/// output, and incremental metrics are summed into the output buffer.
/// Any conversion of metrics is handled by the normalization type
/// `N: MetricNormalize`. Further, distribution metrics have their samples
/// compressed with `compress_distribution` below.
pub struct MetricsBuffer {
    metrics: Option<MetricSet>,
    max_events: usize,
}

impl MetricsBuffer {
    pub const fn new(settings: BatchSize<Self>) -> Self {
        Self::with_capacity(settings.events)
    }

    const fn with_capacity(max_events: usize) -> Self {
        Self {
            metrics: None,
            max_events,
        }
    }
}

impl Batch for MetricsBuffer {
    type Input = Metric;
    type Output = Vec<Metric>;

    fn default_settings<D: SinkBatchSettings>(
        config: BatchConfig<D, Merged>,
    ) -> Result<BatchConfig<D, Merged>, BatchError> {
        config.disallow_max_bytes()
    }

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input> {
        if self.num_items() >= self.max_events {
            PushResult::Overflow(item)
        } else {
            let max_events = self.max_events;
            self.metrics
                .get_or_insert_with(|| MetricSet::with_capacity(max_events))
                .insert(item);

            PushResult::Ok(self.num_items() >= self.max_events)
        }
    }

    fn is_empty(&self) -> bool {
        self.num_items() == 0
    }

    fn fresh(&self) -> Self {
        Self::with_capacity(self.max_events)
    }

    fn finish(self) -> Self::Output {
        self.metrics
            .unwrap_or_else(|| MetricSet::with_capacity(0))
            .0
            .into_iter()
            .map(finish_metric)
            .collect()
    }

    fn num_items(&self) -> usize {
        self.metrics
            .as_ref()
            .map(|metrics| metrics.0.len())
            .unwrap_or(0)
    }
}

fn finish_metric(
    item: (
        MetricSeries,
        (MetricValue, Option<DateTime<Utc>>, EventMetadata),
    ),
) -> Metric {
    let (series, (value, timestamp, metadata)) = item;

    Metric::new_with_metadata(series.name, series.tags, None, value, timestamp, metadata)
}

/// The metrics state trait abstracts how data point normalization is
/// done. Normalization is required to make sure Sources and Sinks are
/// exchanging compatible data structures. For instance, delta gauges
/// produced by Statsd source cannot be directly sent to Datadog API.
/// In this case the buffer will keep the state of a gauge value, and
/// produce absolute values gauges that are well supported by Datadog.
///
/// Another example of normalization is disaggregation of counters. Most
/// sinks would expect we send the delta counters (e.g. how many events
/// occurred during the flush period). And most sources are producing exactly
/// these kind of counters, with Prometheus being a notable exception.
/// If the counter comes already aggregated inside the source, the buffer
/// will compare it's values with the previous known and calculate the
/// delta.
pub trait MetricNormalize {
    /// Apply normalizes the given `metric` using `state` to save any
    /// persistent data between calls. The return value is `None` if the
    /// incoming metric is only used to set a reference state, and
    /// `Some(metric)` otherwise
    fn apply_state(&mut self, state: &mut MetricSet, metric: Metric) -> Option<Metric>;
}

/// This is a simple wrapper for using `MetricNormalize` with a persistent `MetricSet` state,
/// to be used in sinks in `with_flat_map` before sending the events to the `MetricsBuffer`
pub struct MetricNormalizer<N> {
    state: MetricSet,
    normalizer: N,
}

impl<N> MetricNormalizer<N> {
    /// Gets a mutable reference to the current metric state for this normalizer
    pub fn get_state_mut(&mut self) -> &mut MetricSet {
        &mut self.state
    }
}

impl<N: MetricNormalize> MetricNormalizer<N> {
    /// applies normalization ot the given metric, potentially returning an
    /// updated metric
    ///
    /// Depending on the normalizer, a metric may or may not be returned. In the
    /// case of converting a metric from absolute to incremental, a metric must
    /// be seen twice in order to generate an incremental delta, so the first
    /// call for the same metric would return `None` while the second call would
    /// return `Some(...)`.
    pub fn apply(&mut self, metric: Metric) -> Option<Metric> {
        self.normalizer.apply_state(&mut self.state, metric)
    }
}

impl<N: Default> Default for MetricNormalizer<N> {
    fn default() -> Self {
        Self {
            state: MetricSet::default(),
            normalizer: N::default(),
        }
    }
}

impl<N> From<N> for MetricNormalizer<N> {
    fn from(normalizer: N) -> Self {
        Self {
            state: MetricSet::default(),
            normalizer,
        }
    }
}

type MetricEntry = (MetricValue, Option<DateTime<Utc>>, EventMetadata);

/// This is a convenience wrapper for HashMap<MetricSeries, MetricEntry>
/// that provides some extra functionality
pub struct MetricSet(HashMap<MetricSeries, MetricEntry>);

// TODO: this implement is dummy, re-work is needed
impl MetricSet {
    fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    fn insert(&mut self, metric: Metric) {
        let (name, tags, _desc, value, timestamp, metadata) = metric.into_parts();

        self.0
            .insert(MetricSeries { name, tags }, (value, timestamp, metadata));
    }

    fn default() -> Self {
        Self::with_capacity(128)
    }
}
