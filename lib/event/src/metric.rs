use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! gauge_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge($value)
        }
    };
    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge($value)
        }
    };
}

#[macro_export]
macro_rules! sum_metric {
    ($name: expr, $desc: expr, $value: expr, $( $k: expr => $v: expr),* ) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: tags!(
                $($k => $v,)*
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Sum($value.into())
        }
    };

    ($name: expr, $desc: expr, $value: expr) => {
        Metric{
            name: $name.into(),
            description: Some($desc.into()),
            tags: Default::default(),
            unit: None,
            timestamp: 0,
            value: MetricValue::Sum($value)
        }
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

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Metric {
    pub name: String,

    pub description: Option<String>,

    pub tags: BTreeMap<String, String>,

    pub unit: Option<String>,

    pub timestamp: i64,

    pub value: MetricValue,
}

impl Metric {
    pub(crate) fn gauge<N, D, V>(name: N, desc: D, v: V) -> Metric
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

    pub(crate) fn gauge_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
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

    pub(crate) fn sum<N, D, V>(name: N, desc: D, v: V) -> Metric
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

    pub(crate) fn sum_with_tags<N, D, V>(name: N, desc: D, value: V, tags: BTreeMap<String, String>) -> Metric
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