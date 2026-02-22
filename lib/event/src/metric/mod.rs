use std::borrow::Cow;
use std::fmt::{Display, Formatter, Write};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use typesize::TypeSize;

use super::metadata::EventMetadata;
use super::tags::{Key, Tags, Value};
use super::{BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, Finalizable};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
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

                for b in &mut *buckets {
                    if f <= b.upper {
                        b.count += 1;
                    }
                }
            }
            MetricValue::Summary { .. } => {}
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct MetricSeries {
    pub name: Cow<'static, str>,
    pub tags: Tags,
}

impl TypeSize for MetricSeries {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes() + self.tags.allocated_bytes()
    }
}

impl MetricSeries {
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.tags.insert(key.into(), value);
    }
}

impl TypeSize for MetricValue {
    fn allocated_bytes(&self) -> usize {
        match self {
            MetricValue::Sum(_) | MetricValue::Gauge(_) => 0,
            MetricValue::Histogram { buckets, .. } => buckets.len() * size_of::<Bucket>(),
            MetricValue::Summary { quantiles, .. } => quantiles.len() * size_of::<Quantile>(),
        }
    }
}

impl MetricValue {
    pub fn gauge<V: Into<f64>>(v: V) -> MetricValue {
        MetricValue::Gauge(v.into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Metric {
    pub name: Cow<'static, str>,

    pub description: Option<Cow<'static, str>>,

    pub tags: Tags,

    pub value: MetricValue,

    pub timestamp: Option<DateTime<Utc>>,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl TypeSize for Metric {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes()
            + self.tags.allocated_bytes()
            + self.description.allocated_bytes()
            + self.value.allocated_bytes()
            + self.metadata.allocated_bytes()
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
                write_tags(fmt, &self.tags)?;
                write!(fmt, "{}", v)
            }
            MetricValue::Histogram {
                buckets,
                count,
                sum,
            } => {
                let mut tags = self.tags.clone();

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
                tags.remove("le");

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
                let mut tags = self.tags.clone();
                for q in quantiles {
                    tags.insert("quantile", q.quantile.to_string());
                    write!(fmt, "{}", self.name())?;
                    write_tags(fmt, &tags)?;
                    writeln!(fmt, "{}", q.value)?;
                }

                tags.remove("quantile");
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
    ($($typ:ty),+) => {
        $(
            impl IntoF64 for $typ {
                #[inline]
                fn into_f64(self) -> f64 {
                    self as f64
                }
            }
        )+
    };
}

impl_intof64!(usize, u64, i64, f64, u32, i32, f32, u16, i16, u8, i8);

impl IntoF64 for bool {
    #[inline]
    fn into_f64(self) -> f64 {
        if self { 1.0 } else { 0.0 }
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
        name: impl Into<Cow<'static, str>>,
        description: Option<Cow<'static, str>>,
        tags: Tags,
        ts: DateTime<Utc>,
        value: MetricValue,
    ) -> Self {
        Self {
            name: name.into(),
            tags,
            description,
            timestamp: Some(ts),
            value,
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn new_with_metadata(
        name: Cow<'static, str>,
        tags: Tags,
        description: Option<Cow<'static, str>>,
        value: MetricValue,
        timestamp: Option<DateTime<Utc>>,
        metadata: EventMetadata,
    ) -> Self {
        Self {
            name,
            tags,
            description,
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
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        V: IntoF64,
    {
        Self {
            name: name.into(),
            tags: Tags::default(),
            description: Some(desc.into()),
            timestamp: None,
            value: MetricValue::Gauge(v.into_f64()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn gauge_with_tags<N, D, V, A>(name: N, desc: D, value: V, tags: A) -> Metric
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        V: IntoF64,
        A: Into<Tags>,
    {
        Self {
            name: name.into(),
            tags: tags.into(),
            description: Some(desc.into()),
            timestamp: None,
            value: MetricValue::Gauge(value.into_f64()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn sum<N, D, V>(name: N, desc: D, v: V) -> Metric
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        V: IntoF64,
    {
        Self {
            name: name.into(),
            tags: Tags::default(),
            description: Some(desc.into()),
            timestamp: None,
            value: MetricValue::Sum(v.into_f64()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn sum_with_tags<N, D, V, A>(name: N, desc: D, value: V, tags: A) -> Metric
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        V: IntoF64,
        A: Into<Tags>,
    {
        Self {
            name: name.into(),
            tags: tags.into(),
            description: Some(desc.into()),
            timestamp: None,
            value: MetricValue::Sum(value.into_f64()),
            metadata: EventMetadata::default(),
        }
    }

    #[inline]
    pub fn histogram<N, D, C, S>(name: N, desc: D, count: C, sum: S, buckets: Vec<Bucket>) -> Metric
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            name: name.into(),
            tags: Tags::default(),
            description: Some(desc.into()),
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
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        A: Into<Tags>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            name: name.into(),
            tags: tags.into(),
            description: Some(desc.into()),
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
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        C: Into<u64>,
        S: IntoF64,
    {
        Self {
            name: name.into(),
            tags: Tags::default(),
            description: Some(desc.into()),
            timestamp: None,
            metadata: EventMetadata::default(),
            value: MetricValue::Summary {
                count: count.into(),
                sum: sum.into_f64(),
                quantiles,
            },
        }
    }

    #[inline]
    pub fn summary_with_tags<N, D, C, S, A>(
        name: N,
        desc: D,
        count: C,
        sum: S,
        quantiles: Vec<Quantile>,
        tags: A,
    ) -> Metric
    where
        N: Into<Cow<'static, str>>,
        D: Into<Cow<'static, str>>,
        C: Into<u64>,
        S: IntoF64,
        A: Into<Tags>,
    {
        Self {
            name: name.into(),
            tags: tags.into(),
            description: Some(desc.into()),
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

    #[allow(clippy::type_complexity)]
    #[inline]
    pub fn into_parts(
        self,
    ) -> (
        Cow<'static, str>,
        Tags,
        Option<Cow<'static, str>>,
        MetricValue,
        Option<DateTime<Utc>>,
        EventMetadata,
    ) {
        (
            self.name,
            self.tags,
            self.description,
            self.value,
            self.timestamp,
            self.metadata,
        )
    }

    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<Cow<'static, str>>) {
        self.name = name.into();
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
        &self.tags
    }

    #[inline]
    pub fn tags_mut(&mut self) -> &mut Tags {
        &mut self.tags
    }

    #[inline]
    #[must_use]
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = tags;
        self
    }

    #[inline]
    pub fn tag_value(&self, name: &str) -> Option<&Value> {
        self.tags.get(name)
    }

    #[inline]
    pub fn insert_tag(&mut self, key: impl Into<Key>, value: impl Into<Value>) {
        self.tags.insert(key.into(), value);
    }

    #[inline]
    pub fn remote_tag(&mut self, key: &Key) -> Option<Value> {
        self.tags.remove(key)
    }

    #[inline]
    #[must_use]
    pub fn with_desc(mut self, desc: Option<Cow<'static, str>>) -> Self {
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
        assert_eq!(m.description, Some("desc".into()));
        assert_eq!(m.value, MetricValue::Gauge(1.0));
    }

    #[test]
    fn test_sum() {
        let m = Metric::sum("name", "desc", 1);
        assert_eq!(m.name(), "name");
        assert_eq!(m.description, Some("desc".into()));
        assert_eq!(m.value, MetricValue::Sum(1.0));

        let m = Metric::sum_with_tags("name", "desc", 2, tags!("foo" => "bar"));
        assert_eq!(m.name(), "name");
        assert_eq!(m.description, Some("desc".into()));
        assert_eq!(m.value, MetricValue::Sum(2.0));
    }
}
