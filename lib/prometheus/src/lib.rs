mod error;
mod text;

use std::collections::BTreeMap;

use indexmap::IndexMap;

// Re-export
pub use error::Error;
pub use text::parse_text;

pub const METRIC_NAME_LABEL: &str = "__name__";

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/prometheus.rs"));

    pub use metric_metadata::MetricType;

    impl MetricType {
        pub fn as_str(&self) -> &'static str {
            match self {
                MetricType::Counter => "counter",
                MetricType::Gauge => "gauge",
                MetricType::Histogram => "histogram",
                MetricType::Summary => "summary",
                MetricType::Gaugehistogram => "gaugehistogram",
                MetricType::Info => "info",
                MetricType::Stateset => "stateset",
                MetricType::Unknown => "unknown",
            }
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

    fn metric_kind(&self) -> MetricKind {
        match self {
            Self::Counter { .. } => MetricKind::Counter,
            Self::Gauge { .. } => MetricKind::Gauge,
            Self::Histogram { .. } => MetricKind::Histogram,
            Self::Summary { .. } => MetricKind::Summary,
            Self::Untyped { .. } => MetricKind::Untyped,
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
            Self::Counter(ref mut metrics)
            | Self::Gauge(ref mut metrics)
            | Self::Untyped(ref mut metrics) => {
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

            Self::Histogram(ref mut metrics) => match suffix {
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

            Self::Summary(ref mut metrics) => match suffix {
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

    fn push(&mut self, gk: GroupKey, value: f64) {
        match self {
            GroupKind::Gauge(m) | GroupKind::Counter(m) | GroupKind::Untyped(m) => {
                m.insert(gk, value.into());
            }
            _ => {}
        };
    }

    fn push_histogram(&mut self, key: GroupKey, value: HistogramMetric) {
        if let GroupKind::Histogram(m) = self {
            m.insert(key, value);
        }
    }

    fn push_summary(&mut self, key: GroupKey, value: SummaryMetric) {
        if let GroupKind::Summary(m) = self {
            m.insert(key, value);
        }
    }
}

fn matching_group<T: Default>(values: &mut MetricMap<T>, group: GroupKey) -> &mut T {
    values.entry(group).or_insert_with(T::default)
}

fn try_f64_to_u32(f: f64) -> Result<u32, Error> {
    if 0.0 <= f && f <= u32::MAX as f64 {
        Ok(f as u32)
    } else {
        Err(Error::ValueOutOfRange { value: f })
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

    fn insert_sample(
        &mut self,
        name: &str,
        labels: &BTreeMap<String, String>,
        sample: proto::Sample,
    ) -> Result<(), Error> {
        let (_, basename, group) = self.get_group(name);
        if let Some(metric) = group.try_push(
            basename.len(),
            Metric {
                name: name.into(),
                labels: labels.clone(),
                value: sample.value,
                timestamp: Some(sample.timestamp),
            },
        )? {
            let key = GroupKey {
                timestamp: metric.timestamp,
                labels: metric.labels,
            };

            let group = GroupKind::new_untyped(key, metric.value);
            self.0.insert(metric.name, group);
        }

        Ok(())
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

impl From<proto::MetricType> for MetricKind {
    fn from(kind: proto::MetricType) -> Self {
        use proto::MetricType::*;

        match kind {
            Counter => MetricKind::Counter,
            Gauge => MetricKind::Gauge,
            Histogram => MetricKind::Histogram,
            Gaugehistogram => MetricKind::Histogram,
            Summary => MetricKind::Summary,
            _ => MetricKind::Untyped,
        }
    }
}

pub fn parse_request(req: proto::WriteRequest) -> Result<Vec<MetricGroup>, Error> {
    let mut groups = MetricGroupSet::default();

    for metadata in req.metadata {
        let name = metadata.metric_family_name;
        let kind = proto::MetricType::from_i32(metadata.r#type)
            .unwrap_or(proto::MetricType::Unknown)
            .into();

        groups.insert_metadata(name, kind)?;
    }

    for timeseries in req.timeseries {
        let mut labels: BTreeMap<String, String> = timeseries
            .labels
            .into_iter()
            .map(|label| (label.name, label.value))
            .collect();
        let name = match labels.remove(METRIC_NAME_LABEL) {
            Some(name) => name,
            None => return Err(Error::RequestNoNameLabel),
        };

        for sample in timeseries.samples {
            groups.insert_sample(&name, &labels, sample)?;
        }
    }

    Ok(groups.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    macro_rules! match_group {
        ($group: expr, $name: literal, $kind:ident => $inner:expr) => {{
            assert_eq!($group.name, $name);
            let inner = $inner;
            match &$group.metrics {
                GroupKind::$kind(metrics) => inner(metrics),
                _ => panic!("Invalid metric group type"),
            }
        }};
    }

    macro_rules! labels {
        () => { BTreeMap::new() };
        ($($name:ident => $value:literal), *) => {{
            let mut result = BTreeMap::< String, String>::new();
            $ (result.insert(stringify ! ( $ name).into(), $ value.to_string()); ) *
            result
        }};
    }

    macro_rules! simple_metric {
        ($timestamp:expr, $labels:expr, $value:expr) => {
            (
                &GroupKey {
                    timestamp: $timestamp,
                    labels: $labels,
                },
                &SimpleMetric { value: $value },
            )
        };
    }

    #[test]
    fn test_parse_text() {
        // Untyped not supported
        let input = r##"
# HELP http_requests_total The total number of HTTP requests.
# TYPE http_requests_total counter
http_requests_total{method="post",code="200"} 1027 1395066363000
http_requests_total{method="post",code="400"}    3 1395066363000

# Escaping in label values:
# msdos_file_access_time_seconds{path="C:\\DIR\\FILE.TXT",error="Cannot find file:\n\"FILE.TXT\""} 1.458255915e9

# Minimalistic line:
# metric_without_timestamp_and_labels 12.47

# A weird metric from before the epoch:
# something_weird{problem="division by zero"} +Inf -3982045

# A histogram, which has a pretty complex representation in the text format:
# HELP http_request_duration_seconds A histogram of the request duration.
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.05"} 24054
http_request_duration_seconds_bucket{le="0.1"} 33444
http_request_duration_seconds_bucket{le="0.2"} 100392
http_request_duration_seconds_bucket{le="0.5"} 129389
http_request_duration_seconds_bucket{le="1"} 133988
http_request_duration_seconds_bucket{le="+Inf"} 144320
http_request_duration_seconds_sum 53423
http_request_duration_seconds_count 144320

# Finally a summary, which has a complex representation, too:
# HELP rpc_duration_seconds A summary of the RPC duration in seconds.
# TYPE rpc_duration_seconds summary
rpc_duration_seconds{quantile="0.01"} 3102
rpc_duration_seconds{quantile="0.05"} 3272
rpc_duration_seconds{quantile="0.5"} 4773
rpc_duration_seconds{quantile="0.9"} 9001
rpc_duration_seconds{quantile="0.99"} 76656
rpc_duration_seconds_sum 1.7560473e+07
rpc_duration_seconds_count 2693
"##;

        let output = parse_text(input).unwrap();
        assert_eq!(output.len(), 3);
        match_group!(output[0], "http_requests_total", Counter => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 2);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066363000), labels!(method => "post", code => 200), 1027.0)
            );
            assert_eq!(
                metrics.get_index(1).unwrap(),
                simple_metric!(Some(1395066363000), labels!(method => "post", code => 400), 3.0)
            );
        });
        /*
                match_group!(output[1], "msdos_file_access_time_seconds", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(metrics.get_index(0).unwrap(), simple_metric!(
                        None,
                        labels!(path => "C:\\DIR\\FILE.TXT", error => "Cannot find file:\n\"FILE.TXT\""),
                        1.458255915e9
                    ));
                });

                match_group!(output[2], "metric_without_timestamp_and_labels", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(metrics.get_index(0).unwrap(), simple_metric!(None, labels!(), 12.47));
                });

                match_group!(output[3], "something_weird", Untyped => |metrics: &MetricMap<SimpleMetric>| {
                    assert_eq!(metrics.len(), 1);
                    assert_eq!(
                        metrics.get_index(0).unwrap(),
                        simple_metric!(Some(-3982045), labels!(problem => "division by zero"), f64::INFINITY)
                    );
                });
        */
        match_group!(output[1], "http_request_duration_seconds", Histogram => |metrics: &MetricMap<HistogramMetric>| {
            assert_eq!(metrics.len(), 1, "length not match");
            assert_eq!(metrics.get_index(0).unwrap(), (
                &GroupKey {
                    timestamp: None,
                    labels: labels!(),
                },
                &HistogramMetric {
                    buckets: vec![
                        HistogramBucket { bucket: 0.05, count: 24054 },
                        HistogramBucket { bucket: 0.1, count: 33444 },
                        HistogramBucket { bucket: 0.2, count: 100392 },
                        HistogramBucket { bucket: 0.5, count: 129389 },
                        HistogramBucket { bucket: 1.0, count: 133988 },
                        HistogramBucket { bucket: f64::INFINITY, count: 144320 },
                    ],
                    count: 144320,
                    sum: 53423.0,
                },
            ));
        });

        match_group!(output[2], "rpc_duration_seconds", Summary => |metrics: &MetricMap<SummaryMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(metrics.get_index(0).unwrap(), (
                &GroupKey {
                    timestamp: None,
                    labels: labels!(),
                },
                &SummaryMetric {
                    quantiles: vec![
                        SummaryQuantile { quantile: 0.01, value: 3102.0 },
                        SummaryQuantile { quantile: 0.05, value: 3272.0 },
                        SummaryQuantile { quantile: 0.5, value: 4773.0 },
                        SummaryQuantile { quantile: 0.9, value: 9001.0 },
                        SummaryQuantile { quantile: 0.99, value: 76656.0 },
                    ],
                    count: 2693,
                    sum: 1.7560473e+07,
                },
            ));
        });
    }

    #[test]
    fn test_f64_to_u32() {
        let value = -1.0;
        let err = try_f64_to_u32(value).unwrap_err();
        assert_eq!(err, Error::ValueOutOfRange { value });

        let value = u32::MAX as f64 + 1.0;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange { value });

        let value = f64::NAN;
        let error = try_f64_to_u32(value).unwrap_err();
        assert!(matches!(error, Error::ValueOutOfRange { value } if value.is_nan()));

        let value = f64::INFINITY;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange { value });

        let value = f64::NEG_INFINITY;
        let error = try_f64_to_u32(value).unwrap_err();
        assert_eq!(error, Error::ValueOutOfRange { value });

        assert_eq!(try_f64_to_u32(0.0).unwrap(), 0);
        assert_eq!(try_f64_to_u32(u32::MAX as f64).unwrap(), u32::MAX);
    }

    macro_rules! write_request {
        (
            [ $( $name:literal = $type:ident ),* ],
            [
                $( [ $( $label:ident => $value:literal), * ] => [ $( $sample:literal @ $timestamp:literal ),* ]),*
            ]
        ) => {
            proto::WriteRequest {
                metadata: vec![
                    $( proto::MetricMetadata {
                        r#type: proto::MetricType::$type as i32,
                        metric_family_name: $name.into(),
                        help: String::default(),
                        unit: String::default(),
                    }, )*
                ],
                timeseries: vec![ $(proto::TimeSeries {
                    labels: vec![ $( proto::Label {
                        name: stringify!($label).into(),
                        value: $value.to_string(),
                    }, )* ],
                    samples: vec![
                        $( proto::Sample { value: $sample as f64, timestamp: $timestamp as i64},  )*
                    ],
                }, )* ],
            }
        };
    }

    #[test]
    fn parse_request_empty() {
        let parsed = parse_request(write_request!([], [])).unwrap();
        assert!(parsed.is_empty())
    }

    #[test]
    fn parse_request_only_metadata() {
        let parsed = parse_request(write_request!(["one" = Counter, "two" = Gauge], [])).unwrap();
        assert_eq!(parsed.len(), 2);
        match_group!(parsed[0], "one", Counter => |metrics: &MetricMap<SimpleMetric>| {
            assert!(metrics.is_empty());
        });
        match_group!(parsed[1], "two", Gauge => |metrics: &MetricMap<SimpleMetric>| {
            assert!(metrics.is_empty());
        });
    }

    #[test]
    fn parse_request_untyped() {
        let parsed = parse_request(write_request!(
            [],
            [ [__name__ => "one", big => "small"] => [ 123 @ 1395066367500 ]]
        ))
        .unwrap();

        assert_eq!(parsed.len(), 1);
        match_group!(parsed[0], "one", Untyped => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066367500), labels!(big => "small"), 123.0)
            );
        });
    }

    #[test]
    fn parse_request_gauge() {
        let parsed = parse_request(write_request!(
            ["one" = Gauge],
            [
                [__name__ => "one"] => [ 12 @ 1395066367600, 14 @ 1395066367800 ],
                [__name__ => "two"] => [ 13 @ 1395066367700 ]
            ]
        ))
        .unwrap();

        assert_eq!(parsed.len(), 2);
        match_group!(parsed[0], "one", Gauge => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 2);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066367600), labels!(), 12.0),
            );
            assert_eq!(
                metrics.get_index(1).unwrap(),
                simple_metric!(Some(1395066367800), labels!(), 14.0),
            );
        });
        match_group!(parsed[1], "two", Untyped => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066367700), labels!(), 13.0)
            )
        });
    }

    #[test]
    fn parse_request_histogram() {
        let parsed = parse_request(write_request!(
            ["one" = Histogram],
            [
                [__name__ => "one_bucket", le => "1"] => [ 15 @ 1395066367700 ],
                [__name__ => "one_bucket", le => "+Inf"] => [ 19 @ 1395066367700 ],
                [__name__ => "one_count"] => [ 19 @ 1395066367700 ],
                [__name__ => "one_sum"] => [ 12 @ 1395066367700 ],
                [__name__ => "one_total"] => [24 @ 1395066367700]
            ]
        ))
        .unwrap();

        assert_eq!(parsed.len(), 2);
        match_group!(parsed[0], "one", Histogram => |metrics: &MetricMap<HistogramMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(), (
                    &GroupKey {
                        timestamp: Some(1395066367700),
                        labels: labels!()
                    },
                    &HistogramMetric {
                        buckets: vec![
                            HistogramBucket { bucket: 1.0, count: 15 },
                            HistogramBucket { bucket: f64::INFINITY, count: 19 }
                        ],
                        count: 19,
                        sum: 12.0
                    }
                )
            );
        });

        match_group!(parsed[1], "one_total", Untyped => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066367700), labels!(), 24.0)
            );
        })
    }

    #[test]
    fn parse_request_summary() {
        let parsed = parse_request(write_request!(
            ["one" = Summary],
            [
                [__name__ => "one", quantile => "0.5"] => [ 15 @ 1395066367700 ],
                [__name__ => "one", quantile => "0.9"] => [ 19 @ 1395066367700 ],
                [__name__ => "one_count"] => [ 21 @ 1395066367700 ],
                [__name__ => "one_sum"] => [ 12 @ 1395066367700 ],
                [__name__ => "one_total"] => [24 @ 1395066367700]
            ]
        ))
        .unwrap();

        assert_eq!(parsed.len(), 2);
        match_group!(parsed[0], "one", Summary => |metrics: &MetricMap<SummaryMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(), (
                    &GroupKey {
                        timestamp: Some(1395066367700),
                        labels: labels!(),
                    },
                    &SummaryMetric {
                        quantiles: vec![
                            SummaryQuantile { quantile: 0.5, value: 15.0 },
                            SummaryQuantile { quantile: 0.9, value: 19.0 },
                        ],
                        count: 21,
                        sum: 12.0
                    }
                )
            );
        });

        match_group!(parsed[1], "one_total", Untyped => |metrics: &MetricMap<SimpleMetric>| {
            assert_eq!(metrics.len(), 1);
            assert_eq!(
                metrics.get_index(0).unwrap(),
                simple_metric!(Some(1395066367700), labels!(), 24.0)
            );
        })
    }
}
