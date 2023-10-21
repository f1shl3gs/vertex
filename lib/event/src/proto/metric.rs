use std::borrow::Cow;
use std::collections::BTreeMap;

use chrono::TimeZone;

use super::proto_event::{
    metric::Value as PMetricValue, tag_value::Value as PValue, Bucket as PBucket,
    Histogram as PHistogram, Metric as PMetric, Quantile as PQuantile, Summary as PSummary,
    TagValue as PTagValue, TagValueArray as PTagValueArray,
};
use crate::tags::{Array, Key, Tags, Value as TagValue};
use crate::{Metric, MetricValue};

impl From<PTagValueArray> for Array {
    fn from(array: PTagValueArray) -> Self {
        match array.kind {
            0 => Array::Bool(array.bool),
            1 => Array::I64(array.i64),
            2 => Array::F64(array.f64),
            3 => Array::String(array.string.into_iter().map(Cow::from).collect()),
            _ => unreachable!(), // TryFrom is what we need
        }
    }
}

impl From<Array> for PTagValueArray {
    fn from(array: Array) -> Self {
        match array {
            Array::Bool(b) => PTagValueArray {
                kind: 0,
                bool: b,
                ..Default::default()
            },
            Array::I64(i) => PTagValueArray {
                kind: 1,
                i64: i,
                ..Default::default()
            },
            Array::F64(f) => PTagValueArray {
                kind: 2,
                f64: f,
                ..Default::default()
            },
            Array::String(s) => PTagValueArray {
                kind: 3,
                string: s.into_iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
        }
    }
}

impl From<PTagValue> for TagValue {
    fn from(value: PTagValue) -> Self {
        match value.value.unwrap() {
            PValue::Bool(b) => TagValue::Bool(b),
            PValue::I64(i) => TagValue::I64(i),
            PValue::F64(f) => TagValue::F64(f),
            PValue::String(s) => TagValue::String(Cow::from(s)),
            PValue::Array(a) => TagValue::Array(a.into()),
        }
    }
}

impl From<TagValue> for PTagValue {
    fn from(value: TagValue) -> Self {
        let tv = match value {
            TagValue::Bool(b) => PValue::Bool(b),
            TagValue::I64(i) => PValue::I64(i),
            TagValue::F64(f) => PValue::F64(f),
            TagValue::String(s) => PValue::String(s.to_string()),
            TagValue::Array(a) => PValue::Array(a.into()),
        };

        PTagValue { value: Some(tv) }
    }
}

impl From<BTreeMap<String, PTagValue>> for Tags {
    fn from(m: BTreeMap<String, PTagValue>) -> Self {
        m.into_iter()
            .map(|(k, v)| (Key::from(k), TagValue::from(v)))
            .collect()
    }
}

impl From<crate::Bucket> for PBucket {
    fn from(b: crate::Bucket) -> Self {
        Self {
            count: b.count,
            upper: b.upper,
        }
    }
}

impl From<crate::Quantile> for PQuantile {
    fn from(q: crate::Quantile) -> Self {
        Self {
            quantile: q.quantile,
            value: q.value,
        }
    }
}

impl From<PMetric> for Metric {
    fn from(metric: PMetric) -> Self {
        let PMetric {
            name,
            tags,
            description,
            timestamp,
            value,
            ..
        } = metric;

        let timestamp = timestamp
            .map(|ts| chrono::Utc.timestamp_nanos(ts.seconds * 1_000_000_000 + ts.nanos as i64));

        let value = match value.unwrap() {
            PMetricValue::Counter(counter) => MetricValue::Sum(counter.value),
            PMetricValue::Gauge(gauge) => MetricValue::Gauge(gauge.value),
            PMetricValue::Histogram(PHistogram {
                count,
                sum,
                buckets,
            }) => MetricValue::Histogram {
                count,
                sum,
                buckets: buckets
                    .into_iter()
                    .map(|b| crate::Bucket {
                        count: b.count,
                        upper: b.upper,
                    })
                    .collect(),
            },
            PMetricValue::Summary(PSummary {
                count,
                sum,
                quantiles,
            }) => MetricValue::Summary {
                count,
                sum,
                quantiles: quantiles
                    .into_iter()
                    .map(|q| crate::Quantile {
                        quantile: q.quantile,
                        value: q.value,
                    })
                    .collect(),
            },
        };

        let tags = tags
            .into_iter()
            .map(|(k, v)| (Key::from(k), TagValue::from(v)))
            .collect();

        Metric::new(name, Some(description), tags, timestamp.unwrap(), value)
    }
}

impl From<Metric> for PMetric {
    fn from(value: Metric) -> Self {
        todo!()
    }
}
