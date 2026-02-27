use std::collections::BTreeMap;

use super::{
    Error, GroupKey, GroupKind, METRIC_NAME_LABEL, Metric, MetricGroup, MetricGroupSet, MetricKind,
};

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

impl From<proto::MetricType> for MetricKind {
    fn from(kind: proto::MetricType) -> Self {
        use proto::MetricType::{Counter, Gauge, Gaugehistogram, Histogram, Summary};

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

impl MetricGroupSet {
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
}

pub fn parse_request(req: proto::WriteRequest) -> Result<Vec<MetricGroup>, Error> {
    let mut groups = MetricGroupSet::default();

    for metadata in req.metadata {
        let name = metadata.metric_family_name;
        let kind = proto::MetricType::try_from(metadata.r#type)
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

    use crate::{
        HistogramBucket, HistogramMetric, MetricMap, SimpleMetric, SummaryMetric, SummaryQuantile,
    };

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
    fn parse_request_empty() {
        let parsed = parse_request(write_request!([], [])).unwrap();
        assert!(parsed.is_empty())
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
