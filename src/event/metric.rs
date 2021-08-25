use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use getset::{Getters, MutGetters};
use crate::event::value::Value;

pub type Labels = BTreeMap<String, String>;

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialOrd, PartialEq)]
pub struct DataPoint {
    pub tags: BTreeMap<String, String>,
    pub timestamp: u64,
    pub value: MetricValue,
}

#[derive(Clone, Debug, Deserialize, Getters, MutGetters, PartialEq, PartialOrd, Serialize)]
pub struct Metric {
    pub name: String,

    pub description: Option<String>,

    pub unit: Option<String>,

    pub points: Vec<DataPoint>,
}

