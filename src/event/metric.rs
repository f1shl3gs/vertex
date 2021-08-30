use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

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
    pub count: i32,
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
        count: i64,
        sum: f64,
        buckets: Vec<Bucket>,
    },
    Summary {
        count: f64,
        sum: f64,
        quantiles: Vec<Quantile>,
    },
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

    pub timestamp: i64,

    pub value: MetricValue,
}
