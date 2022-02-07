use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Write};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;

use crate::metadata::EventMetadata;
use crate::{BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, Finalizable};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub enum Kind {
    Gauge,
    Sum,
    Histogram,
    Summary,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct Bucket {
    pub upper: f64,
    pub count: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricValue {
    Sum(f64),
    Gauge(f64),
    Histogram {
        count: u64,
        sum: f64,
        buckets: Vec<Bucket>,
    },
    Summary {
        count: u64,
        sum: f64,
        quantiles: Vec<Quantile>,
    },
}

impl MetricValue {
    pub fn add(&mut self, f: impl IntoF64) {
        match self {
            MetricValue::Sum(v) => *v += f.into_f64(),
            _ => unreachable!(),
        }
    }

    pub fn update(&mut self, f: impl IntoF64) {
        match self {
            MetricValue::Sum(v) => *v = f.into_f64(),
            MetricValue::Gauge(v) => *v = f.into_f64(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct MetricSeries {
    pub name: String,
    pub tags: BTreeMap<String, String>,
}

impl ByteSizeOf for MetricSeries {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes() + self.tags.allocated_bytes()
    }
}

impl MetricSeries {
    pub fn insert_tag(
        &mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.tags.insert(key.into(), value.into())
    }
}

impl ByteSizeOf for MetricValue {
    fn allocated_bytes(&self) -> usize {
        match self {
            Self::Sum(_) | Self::Gauge(_) => 0,
            Self::Histogram { .. } => 0,
            Self::Summary { .. } => 0,
        }
    }
}

impl MetricValue {
    pub fn gauge<V: Into<f64>>(v: V) -> MetricValue {
        MetricValue::Gauge(v.into())
    }
}

/// The type alias for an array of `Metric` elements
pub type Metrics = Vec<Metric>;

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Metric {
    #[serde(flatten)]
    pub series: MetricSeries,

    pub description: Option<String>,

    pub unit: Option<String>,

    pub timestamp: Option<DateTime<Utc>>,

    pub value: MetricValue,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl ByteSizeOf for Metric {
    fn allocated_bytes(&self) -> usize {
        self.series.allocated_bytes()
            + self.unit.allocated_bytes()
            + self.description.allocated_bytes()
            + self.value.allocated_bytes()
    }
}

impl Finalizable for Metric {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.metadata.take_finalizers()
    }
}

impl Display for Metric {
    /// Display a metric using something like Prometheus's text format
    ///
    /// ```text
    /// TIMESTAMP NAMESPACE_NAME{TAGS} KIND DATA
    /// ```
    ///
    /// TIMESTAMP is in ISO 8601 format with UTC time zone.
    ///
    /// KIND is either `=` for absolute metrics, or `+` for incremental metrics.
    ///
    /// DATA is dependent on the type of metric, and is a simplified representation
    /// of the data contents. In particular, distributions, histograms, and summaries
    /// are represented as a list of `X@Y` words, where `X` is the rate, count, or quantile,
    /// and `Y` is the value or bucket
    ///
    /// example:
    /// ```text
    /// 2020-08-12T20:23:37.248661343Z vertex_processed_bytes_total{component_kind="sink",component_type="blackhole"} = 6371
    fn fmt(&self, fmt: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(timestamp) = &self.timestamp {
            write!(fmt, "{:?} ", timestamp)?;
        }

        write!(fmt, "{}", self.name())?;

        if !self.series.tags.is_empty() {
            write!(fmt, "{{")?;

            let mut n = 0;
            for (k, v) in self.tags() {
                n += 1;
                write!(fmt, "{}=\"{}\"", k, v)?;
                if n != self.series.tags.len() {
                    fmt.write_char(',')?;
                }
            }

            write!(fmt, "}}")?;
        }

        match self.value {
            MetricValue::Sum(v) | MetricValue::Gauge(v) => {
                write!(fmt, " {}", v)
            }
            _ => Ok(()),
        }
    }
}

impl EventDataEq for Metric {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.value == other.value
            && self.timestamp == other.timestamp
            && self.tags() == other.tags()
            && self.name() == other.name()
            && self.description == other.description
    }
}

pub trait IntoF64 {
    fn into_f64(self) -> f64;
}

macro_rules! impl_intof64 {
    ($typ:ident) => {
        impl IntoF64 for $typ {
            #[inline]
            fn into_f64(self) -> f64 {
                self as f64
            }
        }
    };
}

impl_intof64!(usize);
impl_intof64!(i64);
impl_intof64!(u64);
impl_intof64!(f64);
impl_intof64!(u32);
impl_intof64!(i32);
impl_intof64!(f32);
impl_intof64!(i16);
impl_intof64!(i8);
impl_intof64!(u8);

impl IntoF64 for bool {
    #[inline]
    fn into_f64(self) -> f64 {
        if self {
            1.0
        } else {
            0.0
        }
    }
}

impl IntoF64 for std::time::Duration {
    #[inline]
    fn into_f64(self) -> f64 {
        self.as_secs_f64()
    }
}

impl Metric {
    #[inline]
    pub fn new(
        name: impl ToString,
        description: Option<String>,
        tags: BTreeMap<String, String>,
        ts: DateTime<Utc>,
        value: MetricValue,
    ) -> Self {
        Self {
            series: MetricSeries {
                name: name.to_string(),
                tags,
            },
            description,
            unit: None,
            timestamp: Some(ts),
            value,
            metadata: Default::default(),
        }
    }

    pub fn new_with_metadata(
        name: String,
        tags: BTreeMap<String, String>,
        value: MetricValue,
        timestamp: Option<DateTime<Utc>>,
        metadata: EventMetadata,
    ) -> Self {
        Self {
            series: MetricSeries { name, tags },
            description: None,
            unit: None,
            timestamp,
            value,
            metadata,
        }
    }

    #[inline]
    pub fn gauge<N, D, V>(name: N, desc: D, v: V) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: Default::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(v.into_f64()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn gauge_with_tags<N, D, V>(
        name: N,
        desc: D,
        value: V,
        tags: BTreeMap<String, String>,
    ) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags,
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(value.into_f64()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn sum<N, D, V>(name: N, desc: D, v: V) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: Into<f64>,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: Default::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(v.into()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn sum_with_tags<N, D, V>(
        name: N,
        desc: D,
        value: V,
        tags: BTreeMap<String, String>,
    ) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags,
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(value.into_f64()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn histogram<N, D, C, S>(name: N, desc: D, count: C, sum: S, buckets: Vec<Bucket>) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: Default::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: Default::default(),
            value: MetricValue::Histogram {
                count: count.into(),
                sum: sum.into_f64(),
                buckets,
            },
        }
    }

    #[inline]
    pub fn histogram_with_tags<N, D, C, S>(
        name: N,
        desc: D,
        tags: BTreeMap<String, String>,
        count: C,
        sum: S,
        buckets: Vec<Bucket>,
    ) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags,
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: Default::default(),
            value: MetricValue::Histogram {
                count: count.into(),
                sum: sum.into_f64(),
                buckets,
            },
        }
    }

    #[inline]
    pub fn summary<N, D, C, S>(
        name: N,
        desc: D,
        count: C,
        sum: S,
        quantiles: Vec<Quantile>,
    ) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: Default::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: Default::default(),
            value: MetricValue::Summary {
                count: count.into(),
                sum: sum.into_f64(),
                quantiles,
            },
        }
    }

    pub fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    // TODO: add more information
    #[inline]
    pub fn into_parts(
        self,
    ) -> (
        MetricSeries,
        MetricValue,
        Option<DateTime<Utc>>,
        EventMetadata,
    ) {
        (self.series, self.value, self.timestamp, self.metadata)
    }

    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.series.name
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.series.name = name.into();
    }

    #[inline]
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
    }

    #[inline]
    pub fn with_timestamp(mut self, ts: Option<DateTime<Utc>>) -> Self {
        self.timestamp = ts;
        self
    }

    #[inline]
    pub fn tags(&self) -> &BTreeMap<String, String> {
        &self.series.tags
    }

    #[inline]
    pub fn with_tags(mut self, tags: BTreeMap<String, String>) -> Self {
        self.series.tags = tags;
        self
    }

    pub fn has_tag(&self, key: &str) -> bool {
        self.series.tags.contains_key(key)
    }

    #[inline]
    pub fn tag_value(&self, name: &str) -> Option<&str> {
        self.series.tags.get(name).map(|k| k.as_str())
    }

    #[inline]
    pub fn insert_tag(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Option<String> {
        self.series.insert_tag(name.into(), value.into())
    }

    #[inline]
    pub fn with_desc(mut self, desc: Option<String>) -> Self {
        self.description = desc;
        self
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }

    pub fn with_batch_notifier(mut self, batch: &Arc<BatchNotifier>) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    pub fn with_batch_notifier_option(mut self, batch: &Option<Arc<BatchNotifier>>) -> Self {
        self.metadata = self.metadata.with_batch_notifier_option(batch);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tags;

    #[test]
    fn test_gauge() {
        let m = Metric::gauge("name", "desc", 1);
        assert_eq!(m.name(), "name");
        assert_eq!(m.description, Some("desc".to_string()));
        assert_eq!(m.value, MetricValue::Gauge(1.0));
    }

    #[test]
    fn test_sum() {
        let m = Metric::sum("name", "desc", 1);
        assert_eq!(m.name(), "name");
        assert_eq!(m.description, Some("desc".to_string()));
        assert_eq!(m.value, MetricValue::Sum(1.0));

        let m = Metric::sum_with_tags("name", "desc", 2, tags!("foo" => "bar"));
        assert_eq!(m.name(), "name");
        assert_eq!(m.description, Some("desc".to_string()));
        assert_eq!(m.value, MetricValue::Sum(2.0));
    }
}
