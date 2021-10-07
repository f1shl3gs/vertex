use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use crate::ByteSizeOf;

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
            _ => unreachable!()
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

    /// Nanoseconds
    pub timestamp: i64,

    pub value: MetricValue,
}

impl ByteSizeOf for Metric {
    fn allocated_bytes(&self) -> usize {
        let s1 = self.tags
            .iter()
            .fold(0, |acc, (k, v)| acc + k.len() + v.len());

        s1
    }
}

impl Metric {
    pub fn gauge<N, D, V>(name: N, desc: D, v: V) -> Metric
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
            timestamp: 0,
            value: MetricValue::Gauge(v.into()),
        }
    }

    pub fn gauge_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            V: Into<f64>
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags,
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge(value.into()),
        }
    }

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
            timestamp: 0,
            value: MetricValue::Sum(v.into()),
        }
    }

    pub fn sum_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
        where
            N: Into<String>,
            D: Into<String>,
            V: Into<f64>
    {
        Self {
            name: name.into(),
            description: Some(desc.into()),
            tags,
            unit: None,
            timestamp: 0,
            value: MetricValue::Sum(value.into()),
        }
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