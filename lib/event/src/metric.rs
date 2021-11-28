use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Write};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ByteSizeOf, EventFinalizer};
use crate::metadata::EventMetadata;


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
    pub count: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Quantile {
    pub upper: f64,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialOrd, PartialEq)]
pub struct DataPoint {
    pub tags: BTreeMap<String, String>,
    pub timestamp: u64,
    pub value: MetricValue,
}

impl DataPoint {
    pub fn insert(&mut self, k: String, v: String) {
        self.tags.insert(k, v);
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Metric {
    pub name: String,

    pub description: Option<String>,

    pub tags: BTreeMap<String, String>,

    pub unit: Option<String>,

    pub timestamp: Option<DateTime<Utc>>,

    pub value: MetricValue,

    #[serde(skip)]
    metadata: EventMetadata,
}

impl ByteSizeOf for Metric {
    fn allocated_bytes(&self) -> usize {
        let s1 = self.tags
            .iter()
            .fold(0, |acc, (k, v)| acc + k.len() + v.len());

        s1
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

        write!(fmt, "{}", self.name)?;

        if self.tags.len() != 0 {
            write!(fmt, "{{")?;

            let mut n = 0;
            for (k, v) in &self.tags {
                n += 1;
                write!(fmt, "{}=\"{}\"", k, v)?;
                if n != self.tags.len() {
                    fmt.write_char(',')?;
                }
            }

            write!(fmt, "}}")?;
        }

        match self.value {
            MetricValue::Sum(v) | MetricValue::Gauge(v) => {
                write!(fmt, " {}", v)
            }
            _ => {
                Ok(())
            }
        }
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
    pub fn new(name: impl ToString, tags: BTreeMap<String, String>, value: MetricValue) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            tags,
            unit: None,
            timestamp: None,
            value,
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn gauge<N, D, V>(name: N, desc: D, v: V) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            V: IntoF64
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: None,
            value: MetricValue::Gauge(v.into_f64()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn gauge_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            V: IntoF64
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags,
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
            V: Into<f64>
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(v.into()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn sum_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            V: IntoF64
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags,
            unit: None,
            timestamp: None,
            value: MetricValue::Sum(value.into_f64()),
            metadata: Default::default(),
        }
    }

    #[inline]
    pub fn histogram<N, D, C, S>(
        name: N,
        desc: D,
        count: C,
        sum: S,
        buckets: Vec<Bucket>,
    ) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            C: Into<u64>,
            S: IntoF64
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags: Default::default(),
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
        quantiles: Vec<Quantile>
    ) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            C: Into<u64>,
            S: IntoF64
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: None,
            metadata: Default::default(),
            value: MetricValue::Summary {
                count: count.into(),
                sum: sum.into_f64(),
                quantiles
            },
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
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
    pub fn with_tags(mut self, tags: BTreeMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    #[inline]
    pub fn with_desc(mut self, desc: Option<String>) -> Self {
        self.description = desc;
        self
    }

    #[inline]
    pub fn tag_value(&self, name: &str) -> Option<String> {
        self.tags
            .get(name)
            .map(|v| v.to_string())
    }

    #[inline]
    pub fn insert_tag(&mut self, name: impl ToString, value: impl ToString) -> Option<String> {
        self.tags
            .insert(name.to_string(), value.to_string())
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gauge() {
        let m = Metric::gauge("name", "desc", 1);
        assert_eq!(m.name, "name");
        assert_eq!(m.description, Some("desc".to_string()));
        assert_eq!(m.value, MetricValue::Gauge(1.0));
    }
}