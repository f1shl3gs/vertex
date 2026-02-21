use std::borrow::Cow;

use chrono::{DateTime, Utc};
use event::tags::Tags;
use event::{Bucket, Metric, MetricValue, Quantile};
use indexmap::IndexMap;
use prometheus::{GroupKind, METRIC_NAME_LABEL, MetricGroup, proto};

type Labels = Vec<proto::Label>;

pub struct TimeSeries {
    buffer: IndexMap<Labels, Vec<proto::Sample>>,
    metadata: IndexMap<String, proto::MetricMetadata>,
    timestamp: Option<i64>,
}

impl TimeSeries {
    pub fn new() -> Self {
        Self {
            buffer: Default::default(),
            metadata: Default::default(),
            timestamp: None,
        }
    }

    fn default_timestamp(&mut self) -> i64 {
        *self
            .timestamp
            .get_or_insert_with(|| Utc::now().timestamp_millis())
    }

    fn emit_metadata(&mut self, name: &str, fullname: &str, value: &MetricValue) {
        if !self.metadata.contains_key(name) {
            let r#type = prometheus_metric_type(value);
            let metadata = proto::MetricMetadata {
                r#type: r#type as i32,
                metric_family_name: fullname.into(),
                help: name.into(),
                unit: String::new(),
            };

            self.metadata.insert(name.into(), metadata);
        }
    }

    fn emit_value(
        &mut self,
        timestamp: Option<i64>,
        name: &str,
        suffix: Option<&str>,
        value: f64,
        tags: &Tags,
        extra: Option<(&str, String)>,
    ) {
        let timestamp = timestamp.unwrap_or_else(|| self.default_timestamp());
        self.buffer
            .entry(Self::make_labels(tags, name, suffix, extra))
            .or_default()
            .push(proto::Sample { value, timestamp });
    }

    pub fn finish(self) -> proto::WriteRequest {
        let timeseries = self
            .buffer
            .into_iter()
            .map(|(labels, samples)| proto::TimeSeries { labels, samples })
            .collect::<Vec<_>>();

        let metadata = self
            .metadata
            .into_iter()
            .map(|(_, metadata)| metadata)
            .collect();

        proto::WriteRequest {
            timeseries,
            metadata,
        }
    }

    pub fn encode_metric(&mut self, metric: &Metric) {
        let name = metric.name();
        let timestamp = metric.timestamp().map(|ts| ts.timestamp_millis());
        let tags = &metric.tags();
        self.emit_metadata(name, name, &metric.value);

        match &metric.value {
            MetricValue::Sum(value) | MetricValue::Gauge(value) => {
                self.emit_value(timestamp, name, None, *value, tags, None)
            }
            MetricValue::Histogram {
                count,
                sum,
                buckets,
            } => {
                for bucket in buckets {
                    self.emit_value(
                        timestamp,
                        name,
                        Some("_bucket"),
                        bucket.count as f64,
                        tags,
                        Some(("le", bucket.upper.to_string())),
                    );
                }

                self.emit_value(timestamp, name, Some("_sum"), *sum, tags, None);
                self.emit_value(timestamp, name, Some("_count"), *count as f64, tags, None);
            }
            MetricValue::Summary {
                count,
                sum,
                quantiles,
            } => {
                for quantile in quantiles {
                    self.emit_value(
                        timestamp,
                        name,
                        None,
                        quantile.value,
                        tags,
                        Some(("quantile", quantile.quantile.to_string())),
                    )
                }

                self.emit_value(timestamp, name, Some("_sum"), *sum, tags, None);
                self.emit_value(timestamp, name, Some("_count"), *count as f64, tags, None);
            }
        }
    }

    fn make_labels(
        attrs: &Tags,
        name: &str,
        suffix: Option<&str>,
        extra: Option<(&str, String)>,
    ) -> Labels {
        let mut attrs = attrs.clone();

        // Each Prometheus metric is grouped by its labels, which contains all the labels
        // from the source metric, plus the name label for the actual metric name. For
        // convenience below, an optional extra tag is added.
        let name = match suffix {
            Some(suffix) => [name, suffix].join(""),
            None => name.to_string(),
        };

        attrs.insert(METRIC_NAME_LABEL, name);

        if let Some((name, value)) = extra {
            attrs.insert(name.to_string(), value);
        }

        // Extract the labels into a vec and sort to produce a consistent key for the
        // buffer
        let mut labels = attrs
            .into_iter()
            .map(|(key, value)| proto::Label {
                name: key.to_string(),
                value: value.to_string(),
            })
            .collect::<Labels>();

        labels.sort();
        labels
    }
}

const fn prometheus_metric_type(value: &MetricValue) -> proto::MetricType {
    match value {
        MetricValue::Sum(_) => proto::MetricType::Counter,
        MetricValue::Gauge(_) => proto::MetricType::Gauge,
        MetricValue::Histogram { .. } => proto::MetricType::Histogram,
        MetricValue::Summary { .. } => proto::MetricType::Summary,
    }
}

#[cfg(any(
    feature = "sources-prometheus_pushgateway",
    feature = "sources-prometheus_scrape",
    feature = "sources-prometheus_textfile",
))]
pub fn convert_metrics(groups: Vec<MetricGroup>) -> Vec<Metric> {
    fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match timestamp {
            None => Some(default),
            Some(timestamp) => DateTime::<Utc>::from_timestamp_millis(timestamp),
        }
    }

    let start = Utc::now();
    let mut metrics = Vec::with_capacity(groups.len());

    for group in groups {
        let name = Cow::<'static, str>::Owned(group.name);
        let description = Cow::<'static, str>::Owned(group.description);
        let grouped = group.metrics;

        match grouped {
            GroupKind::Counter(set) => {
                for (key, metric) in set {
                    let metric = Metric::sum(name.clone(), description.clone(), metric.value)
                        .with_tags(key.labels.into())
                        .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(metric);
                }
            }
            GroupKind::Gauge(set) | GroupKind::Untyped(set) => {
                for (key, metric) in set {
                    let metric = Metric::gauge(name.clone(), description.clone(), metric.value)
                        .with_tags(key.labels.into())
                        .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(metric);
                }
            }
            GroupKind::Summary(set) => {
                for (key, metric) in set {
                    let metric = Metric::summary(
                        name.clone(),
                        description.clone(),
                        metric.count,
                        metric.sum,
                        metric
                            .quantiles
                            .iter()
                            .map(|q| Quantile {
                                quantile: q.quantile,
                                value: q.value,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_tags(key.labels.into())
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(metric);
                }
            }
            GroupKind::Histogram(set) => {
                for (key, metric) in set {
                    let metric = Metric::histogram(
                        name.clone(),
                        description.clone(),
                        metric.count,
                        metric.sum,
                        metric
                            .buckets
                            .iter()
                            .map(|b| Bucket {
                                upper: b.bucket,
                                count: b.count,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_tags(key.labels.into())
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(metric);
                }
            }
        }
    }

    metrics
}
