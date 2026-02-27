mod pb;
mod text;

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::num::{ParseFloatError, ParseIntError};

use indexmap::IndexMap;

pub use pb::{parse_request, proto};
pub use text::parse_text;

pub const METRIC_NAME_LABEL: &str = "__name__";

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Error {
    InvalidType(String),
    InvalidMetric(String),

    MissingValue,
    MissingQuantile(String),
    InvalidQuantile { line: String, err: ParseFloatError },
    MissingBucket(String),
    InvalidBucket { line: String, err: ParseFloatError },

    InvalidMetricValue(ParseFloatError),
    InvalidTimestamp(ParseIntError),

    MultipleMetricKinds { name: String },
    RequestNoNameLabel,
    ValueOutOfRange(f64),
}

impl From<ParseFloatError> for Error {
    fn from(err: ParseFloatError) -> Self {
        Self::InvalidMetricValue(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidType(typ) => write!(f, "invalid metric type {typ}"),
            Error::InvalidMetric(name) => write!(f, "invalid metric name {name}"),
            Error::MissingValue => f.write_str("value is not found"),
            Error::MissingQuantile(line) => write!(f, "quantile label is not found in `{line}`"),
            Error::InvalidQuantile { line, err } => {
                write!(f, "invalid quantile label value {line}, {err}")
            }
            Error::MissingBucket(line) => write!(f, "bucket label is not found in `{line}`"),
            Error::InvalidBucket { line, err } => {
                write!(f, "invalid bucket value in {line}, {err}")
            }
            Error::InvalidMetricValue(err) => write!(f, "invalid metric value, {err}"),
            Error::InvalidTimestamp(err) => write!(f, "invalid timestamp {err}"),
            Error::MultipleMetricKinds { name } => write!(f, "metric `{name}` have multiple kind"),
            Error::RequestNoNameLabel => f.write_str("metric name label not found"),
            Error::ValueOutOfRange(value) => write!(f, "metric value `{value}` is out of range"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
    Summary,
    Untyped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metric {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
    pub timestamp: Option<i64>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GroupKey {
    pub timestamp: Option<i64>,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Default, PartialEq)]
pub struct SummaryQuantile {
    pub quantile: f64,
    pub value: f64,
}

#[derive(Debug, Default, PartialEq)]
pub struct SummaryMetric {
    pub quantiles: Vec<SummaryQuantile>,
    pub sum: f64,
    pub count: u32,
}

#[derive(Debug, Default, PartialEq, PartialOrd)]
pub struct HistogramBucket {
    pub bucket: f64,
    pub count: u64,
}

#[derive(Debug, Default, PartialEq)]
pub struct HistogramMetric {
    pub buckets: Vec<HistogramBucket>,
    pub sum: f64,
    pub count: u32,
}

#[derive(Debug, Default, PartialEq)]
pub struct SimpleMetric {
    pub value: f64,
}

impl From<f64> for SimpleMetric {
    fn from(value: f64) -> Self {
        Self { value }
    }
}

pub type MetricMap<T> = IndexMap<GroupKey, T>;

#[derive(Debug)]
pub enum GroupKind {
    Summary(MetricMap<SummaryMetric>),
    Histogram(MetricMap<HistogramMetric>),
    Gauge(MetricMap<SimpleMetric>),
    Counter(MetricMap<SimpleMetric>),
    Untyped(MetricMap<SimpleMetric>),
}

impl GroupKind {
    fn new(kind: MetricKind) -> Self {
        match kind {
            MetricKind::Summary => Self::Summary(IndexMap::default()),
            MetricKind::Histogram => Self::Histogram(IndexMap::default()),
            MetricKind::Gauge => Self::Gauge(IndexMap::default()),
            MetricKind::Counter => Self::Counter(IndexMap::default()),
            MetricKind::Untyped => Self::Untyped(IndexMap::default()),
        }
    }

    fn new_untyped(key: GroupKey, value: f64) -> Self {
        let mut metrics = IndexMap::default();
        metrics.insert(key, SimpleMetric { value });
        Self::Untyped(metrics)
    }

    fn matches_kind(&self, kind: MetricKind) -> bool {
        match self {
            Self::Counter { .. } => kind == MetricKind::Counter,
            Self::Gauge { .. } => kind == MetricKind::Gauge,
            Self::Histogram { .. } => kind == MetricKind::Histogram,
            Self::Summary { .. } => kind == MetricKind::Summary,
            Self::Untyped { .. } => true,
        }
    }

    /// Err(_) if there are irrecoverable error.
    /// Ok(Some(metric)) if this metric belongs to another group.
    /// Ok(None) pushed successfully.
    fn try_push(&mut self, prefix_len: usize, metric: Metric) -> Result<Option<Metric>, Error> {
        let suffix = &metric.name[prefix_len..];
        let mut key = GroupKey {
            timestamp: metric.timestamp,
            labels: metric.labels,
        };
        let value = metric.value;

        match self {
            Self::Counter(metrics) | Self::Gauge(metrics) | Self::Untyped(metrics) => {
                if !suffix.is_empty() {
                    return Ok(Some(Metric {
                        name: metric.name,
                        timestamp: key.timestamp,
                        labels: key.labels,
                        value,
                    }));
                }

                metrics.insert(key, SimpleMetric { value });
            }

            Self::Histogram(metrics) => match suffix {
                "_bucket" => {
                    let bucket = key
                        .labels
                        .remove("le")
                        .ok_or_else(|| Error::MissingBucket(metric.name.clone()))?;
                    let bucket = bucket.parse::<f64>().map_err(|err| Error::InvalidBucket {
                        line: metric.name,
                        err,
                    })?;
                    let count = metric.value as u64;
                    matching_group(metrics, key)
                        .buckets
                        .push(HistogramBucket { bucket, count });
                }

                "_sum" => {
                    let sum = metric.value;
                    matching_group(metrics, key).sum = sum;
                }

                "_count" => {
                    let count = try_f64_to_u32(metric.value)?;
                    matching_group(metrics, key).count = count;
                }
                _ => {
                    return Ok(Some(Metric {
                        name: metric.name,
                        timestamp: key.timestamp,
                        labels: key.labels,
                        value,
                    }));
                }
            },

            Self::Summary(metrics) => match suffix {
                "" => {
                    let quantile = key
                        .labels
                        .remove("quantile")
                        .ok_or_else(|| Error::MissingQuantile(metric.name.clone()))?;
                    let value = metric.value;
                    let quantile =
                        quantile
                            .parse::<f64>()
                            .map_err(|err| Error::InvalidQuantile {
                                line: metric.name,
                                err,
                            })?;

                    matching_group(metrics, key)
                        .quantiles
                        .push(SummaryQuantile { quantile, value })
                }
                "_sum" => {
                    let sum = metric.value;
                    matching_group(metrics, key).sum = sum;
                }
                "_count" => {
                    let count = try_f64_to_u32(metric.value)?;
                    matching_group(metrics, key).count = count;
                }
                _ => {
                    return Ok(Some(Metric {
                        name: metric.name,
                        timestamp: key.timestamp,
                        labels: key.labels,
                        value,
                    }));
                }
            },
        }

        Ok(None)
    }
}

fn matching_group<T: Default>(values: &mut MetricMap<T>, group: GroupKey) -> &mut T {
    values.entry(group).or_default()
}

fn try_f64_to_u32(f: f64) -> Result<u32, Error> {
    if 0.0 <= f && f <= u32::MAX as f64 {
        Ok(f as u32)
    } else {
        Err(Error::ValueOutOfRange(f))
    }
}

#[derive(Debug)]
pub struct MetricGroup {
    pub name: String,
    pub description: String,
    pub metrics: GroupKind,
}

#[derive(Default)]
struct MetricGroupSet(IndexMap<String, GroupKind>);

impl MetricGroupSet {
    fn get_group<'a>(&'a mut self, name: &str) -> (usize, &'a String, &'a mut GroupKind) {
        let len = name.len();
        let name = if self.0.contains_key(name) {
            name
        } else if name.ends_with("_bucket") && self.0.contains_key(&name[..len - 7]) {
            &name[..len - 7]
        } else if name.ends_with("_sum") && self.0.contains_key(&name[..len - 4]) {
            &name[..len - 4]
        } else if name.ends_with("_count") && self.0.contains_key(&name[..len - 6]) {
            &name[..len - 6]
        } else {
            self.0
                .insert(name.into(), GroupKind::new(MetricKind::Untyped));
            name
        };

        self.0.get_full_mut(name).unwrap()
    }

    fn insert_metadata(&mut self, name: String, kind: MetricKind) -> Result<(), Error> {
        match self.0.get(&name) {
            Some(group) if !group.matches_kind(kind) => Err(Error::MultipleMetricKinds { name }),
            Some(_) => Ok(()), // metadata already exists and is the right type
            None => {
                self.0.insert(name, GroupKind::new(kind));
                Ok(())
            }
        }
    }

    fn finish(self) -> Vec<MetricGroup> {
        self.0
            .into_iter()
            .map(|(name, metrics)| MetricGroup {
                name,
                description: "".into(),
                metrics,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn to_u32() {
        let value = -1.0;
        let err = try_f64_to_u32(value).unwrap_err();
        assert_eq!(err, Error::ValueOutOfRange(value));

        let value = u32::MAX as f64 + 1.0;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange(value));

        let value = f64::NAN;
        let error = try_f64_to_u32(value).unwrap_err();
        assert!(matches!(error, Error::ValueOutOfRange (value) if value.is_nan()));

        let value = f64::INFINITY;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange(value));

        let value = f64::NEG_INFINITY;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange(value));

        assert_eq!(try_f64_to_u32(0.0).unwrap(), 0);
        assert_eq!(try_f64_to_u32(u32::MAX as f64).unwrap(), u32::MAX);
    }
}
