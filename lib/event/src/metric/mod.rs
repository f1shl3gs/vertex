use std::fmt::{Display, Formatter, Write};

use chrono::{DateTime, Utc};
use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use crate::metadata::EventMetadata;
use crate::tags::{Key, Tags, Value};
use crate::{BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, Finalizable};

pub const INSTANCE_KEY: Key = Key::from_static_str("instance");
pub const EXPORTED_INSTANCE_KEY: Key = Key::from_static_str("exported_instance");

#[macro_export]
macro_rules! buckets {
    ( $( $limit:expr => $count:expr),* ) => {
        vec![
            $( event::Bucket { upper: $limit, count: $count}, )*
        ]
    };
}

#[macro_export]
macro_rules! quantiles {
    ( $( $q:expr => $value:expr),* ) => {
        vec![
            $( event::Quantile { quantile: $q, value: $value }, )*
        ]
    };
}

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
    pub fn merge(&mut self, f: impl IntoF64) {
        let f = f.into_f64();

        match self {
            MetricValue::Sum(s) => *s += f,
            MetricValue::Gauge(g) => *g = f,
            MetricValue::Histogram {
                buckets,
                count,
                sum,
            } => {
                *count += 1;
                *sum += f;

                for b in buckets.iter_mut() {
                    if f <= b.upper {
                        b.count += 1;
                    }
                }
            }
            MetricValue::Summary { .. } => {}
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct MetricSeries {
    pub name: String,
    pub tags: Tags,
}

impl ByteSizeOf for MetricSeries {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes() + self.tags.allocated_bytes()
    }
}

impl MetricSeries {
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.tags.insert(key, value);
    }
}

impl ByteSizeOf for MetricValue {
    fn allocated_bytes(&self) -> usize {
        // TODO: implement
        0
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
        fn write_tags(fmt: &mut Formatter<'_>, tags: &Tags) -> std::fmt::Result {
            if tags.is_empty() {
                return write!(fmt, " ");
            }

            write!(fmt, "{{")?;
            let mut n = 0;
            for (k, v) in tags {
                n += 1;
                write!(fmt, "{}=\"{}\"", k, v)?;
                if n != tags.len() {
                    fmt.write_char(',')?;
                }
            }

            write!(fmt, "}} ")
        }

        if let Some(timestamp) = &self.timestamp {
            write!(fmt, "{:?} ", timestamp)?;
        }

        match &self.value {
            MetricValue::Gauge(v) | MetricValue::Sum(v) => {
                write!(fmt, "{}", self.name())?;
                write_tags(fmt, &self.series.tags)?;
                write!(fmt, "{}", v)
            }
            MetricValue::Histogram {
                buckets,
                count,
                sum,
            } => {
                let mut tags = self.series.tags.clone();

                for b in buckets {
                    if b.upper == f64::INFINITY {
                        tags.insert("le", "+Inf");
                    } else {
                        tags.insert("le", b.upper.to_string());
                    }

                    write!(fmt, "{}_bucket", self.name())?;
                    write_tags(fmt, &tags)?;
                    writeln!(fmt, "{}", b.count)?;
                }

                // write sum and total
                tags.remove(&"le".into());

                write!(fmt, "{}_sum", self.name())?;
                write_tags(fmt, &tags)?;
                writeln!(fmt, "{}", sum)?;

                write!(fmt, "{}_count", self.name())?;
                write_tags(fmt, &tags)?;
                write!(fmt, "{}", count)
            }
            MetricValue::Summary {
                count,
                sum,
                quantiles,
            } => {
                let mut tags = self.series.tags.clone();
                for q in quantiles {
                    tags.insert("quantile", q.quantile.to_string());
                    write!(fmt, "{}", self.name())?;
                    write_tags(fmt, &tags)?;
                    writeln!(fmt, "{}", q.value)?;
                }

                tags.remove(&"quantile".into());
                write!(fmt, "{}_sum", self.name())?;
                write_tags(fmt, &tags)?;
                writeln!(fmt, "{}", sum)?;

                write!(fmt, "{}_count", self.name())?;
                write_tags(fmt, &tags)?;
                write!(fmt, "{}", count)
            }
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
        name: impl Into<String>,
        description: Option<String>,
        tags: Tags,
        ts: DateTime<Utc>,
        value: MetricValue,
    ) -> Self {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags,
            },
            description,
            unit: None,
            timestamp: Some(ts),
            value,
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn new_with_metadata(
        name: String,
        tags: Tags,
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
    pub fn value(&self) -> &MetricValue {
        &self.value
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
                tags: Tags::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(v.into_f64()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn gauge_with_tags<N, D, V, A>(name: N, desc: D, value: V, tags: A) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: IntoF64,
        A: Into<Tags>,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: tags.into(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(value.into_f64()),
            metadata: EventMetadata::default(),
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
                tags: Tags::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(v.into()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn sum_with_tags<N, D, V, A>(name: N, desc: D, value: V, tags: A) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        V: IntoF64,
        A: Into<Tags>,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: tags.into(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(value.into_f64()),
            metadata: EventMetadata::default(),
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
                tags: Tags::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: EventMetadata::default(),
            value: MetricValue::Histogram {
                count: count.into(),
                sum: sum.into_f64(),
                buckets,
            },
        }
    }

    #[inline]
    pub fn histogram_with_tags<N, D, A, C, S>(
        name: N,
        desc: D,
        tags: A,
        count: C,
        sum: S,
        buckets: Vec<Bucket>,
    ) -> Metric
    where
        N: Into<String>,
        D: Into<String>,
        A: Into<Tags>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            series: MetricSeries {
                name: name.into(),
                tags: tags.into(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: EventMetadata::default(),
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
                tags: Tags::default(),
            },
            description: Some(desc.into()),
            unit: None,
            timestamp: None,
            metadata: EventMetadata::default(),
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
    #[must_use]
    pub fn with_timestamp(mut self, ts: Option<DateTime<Utc>>) -> Self {
        self.timestamp = ts;
        self
    }

    #[inline]
    pub fn tags(&self) -> &Tags {
        &self.series.tags
    }

    #[inline]
    pub fn tags_mut(&mut self) -> &mut Tags {
        &mut self.series.tags
    }

    #[inline]
    #[must_use]
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.series.tags = tags;
        self
    }

    #[inline]
    pub fn has_tag(&self, key: &str) -> bool {
        // TODO: avoid allocation
        self.series.tags.contains_key(key.to_string())
    }

    #[inline]
    pub fn tag_value(&self, name: &str) -> Option<&Value> {
        self.series.tags.get(&Key::from(name.to_string()))
    }

    #[inline]
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.series.insert_tag(key, value);
    }

    #[inline]
    pub fn remote_tag(&mut self, key: &Key) -> Option<Value> {
        self.series.tags.remove(key)
    }

    #[inline]
    #[must_use]
    pub fn with_desc(mut self, desc: Option<String>) -> Self {
        self.description = desc;
        self
    }

    #[inline]
    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }

    #[inline]
    #[must_use]
    pub fn with_batch_notifier(mut self, batch: &BatchNotifier) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    #[inline]
    #[must_use]
    pub fn with_batch_notifier_option(mut self, batch: &Option<BatchNotifier>) -> Self {
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
