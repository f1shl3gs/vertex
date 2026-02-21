use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::pin::Pin;
use std::time::Duration;

use async_stream::stream;
use chrono::Utc;
use configurable::{Configurable, configurable_component};
use event::log::{LogRecord, OwnedTargetPath, Value};
use event::tags::{Key, Tags};
use event::{Bucket, EventMetadata, Events, Metric, MetricSeries, MetricValue};
use framework::config::{
    InputType, OutputType, TransformConfig, TransformContext, default_interval,
};
use framework::{TaskTransform, Transform};
use futures::{Stream, StreamExt};
use metrics::Counter;
use serde::{Deserialize, Serialize};

fn default_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ]
}

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase", tag = "type")]
enum MetricType {
    Counter {
        /// Controls how to increase the counter.
        increment_by_value: bool,
    },
    Gauge,
    Histogram {
        /// Specify histogram buckets.
        /// Available for "histogram" only
        /// Default: 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
        #[serde(default = "default_buckets")]
        buckets: Vec<f64>,
    },
}

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
struct MetricConfig {
    /// Metric name, it's highly recommend to see
    /// https://prometheus.io/docs/practices/naming/
    name: String,

    /// Which field to extract values
    ///
    /// Path is support too, e.g. field: value.i64
    field: OwnedTargetPath,

    /// Tags to set, this field is not required,
    /// but is is recommend to set some tags to identify your metrics.
    ///
    /// The key is the tag key for metrics, the value is the path to the log field.
    #[serde(default)]
    tags: BTreeMap<Key, OwnedTargetPath>,

    #[serde(flatten)]
    typ: MetricType,
}

impl MetricConfig {
    fn build_series_and_value(&self, log: &LogRecord) -> Option<(MetricSeries, f64)> {
        let MetricConfig { name, field, .. } = self;

        let parse_value = match self.typ {
            MetricType::Counter { increment_by_value } => increment_by_value,
            _ => true,
        };

        let value = match log.get(field)? {
            Value::Integer(i) => *i as f64,
            Value::Float(f) => *f,
            Value::Bytes(b) => {
                if parse_value {
                    String::from_utf8_lossy(b.as_ref()).parse().ok()?
                } else {
                    1.0
                }
            }
            _ => return None,
        };

        let mut tags = Tags::with_capacity(self.tags.len());
        for (key, path) in &self.tags {
            if let Some(value) = log.get(path) {
                tags.insert(key.clone(), value.to_string_lossy().to_string());
            }
        }

        Some((
            MetricSeries {
                name: Cow::<'static, str>::Owned(name.to_string()),
                tags,
            },
            value,
        ))
    }

    fn new_metric_value(&self, value: f64) -> MetricValue {
        match &self.typ {
            MetricType::Counter { .. } => MetricValue::Sum(value),
            MetricType::Gauge => MetricValue::Gauge(value),
            MetricType::Histogram { buckets } => MetricValue::Histogram {
                count: 1,
                sum: value,
                buckets: buckets
                    .iter()
                    .map(|upper| Bucket {
                        upper: *upper,
                        count: u64::from(value <= *upper),
                    })
                    .collect(),
            },
        }
    }
}

#[configurable_component(transform, name = "metricalize")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The interval between flushes.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// A table of key/value pairs representing the keys to be added to the event.
    ///
    /// Available values:
    /// counter:     Counter
    /// gauge:       Gauge
    /// histogram:   Histogram
    metrics: Vec<MetricConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "metricalize")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let agg = Metricalize::new(self.interval, self.metrics.clone());
        Ok(Transform::event_task(agg))
    }

    fn input(&self) -> InputType {
        InputType::log()
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

impl TaskTransform for Metricalize {
    fn transform(
        mut self: Box<Self>,
        mut input_rx: Pin<Box<dyn Stream<Item = Events> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Events> + Send>> {
        let mut ticker = tokio::time::interval(self.interval);

        Box::pin(stream! {
            let mut output = Vec::new();
            let mut done = false;

            while !done {
                tokio::select! {
                    _ = ticker.tick() => {
                        self.flush_into(&mut output);
                        yield Events::from(output);
                        output = Vec::new();
                    },

                    maybe_events = input_rx.next() => {
                        match maybe_events {
                            Some(events) => self.record(events),
                            None => {
                                self.flush_into(&mut output);
                                yield Events::from(output);
                                output = Vec::new();

                                done = true;
                            }
                        }
                    }
                }
            }
        })
    }
}

type MetricEntry = (MetricValue, EventMetadata);

struct Metricalize {
    interval: Duration,
    configs: Vec<MetricConfig>,
    states: HashMap<MetricSeries, MetricEntry>,

    // metrics
    failures: Counter,
}

impl Metricalize {
    fn new(interval: Duration, configs: Vec<MetricConfig>) -> Self {
        let failures = metrics::register_counter(
            "metricalize_failed_total",
            "The amount of failures of metricalizing",
        )
        .recorder(&[]);

        Self {
            interval,
            configs,
            states: Default::default(),
            failures,
        }
    }

    fn record(&mut self, events: Events) {
        if let Events::Logs(logs) = events {
            logs.iter().for_each(|log| {
                let metadata = log.metadata();

                for config in &self.configs {
                    match config.build_series_and_value(log) {
                        Some((series, value)) => {
                            match self.states.entry(series) {
                                Entry::Occupied(mut entry) => {
                                    let existing = entry.get_mut();

                                    // In order to update the new and old kind must match
                                    match (&existing.0, &config.typ) {
                                        (MetricValue::Sum(_), MetricType::Counter { .. })
                                        | (MetricValue::Gauge(_), MetricType::Gauge)
                                        | (
                                            MetricValue::Histogram { .. },
                                            MetricType::Histogram { .. },
                                        ) => {
                                            existing.0.merge(value);
                                            existing.1.merge(metadata.clone());
                                        }
                                        _ => {
                                            *existing =
                                                (config.new_metric_value(value), metadata.clone());
                                            self.failures.inc(1);
                                        }
                                    }
                                }

                                Entry::Vacant(entry) => {
                                    entry
                                        .insert((config.new_metric_value(value), metadata.clone()));
                                }
                            }
                        }
                        None => self.failures.inc(1),
                    }
                }
            })
        }
    }

    fn flush_into(&mut self, output: &mut Vec<Metric>) {
        let now = Utc::now();

        for (series, entry) in self.states.iter_mut() {
            let metadata = entry.1.take_finalizers().into();

            let metric = Metric::new_with_metadata(
                series.name.clone(),
                series.tags.clone(),
                None,
                entry.0.clone(),
                Some(now),
                metadata,
            );

            output.push(metric);
        }
    }
}

#[cfg(test)]
mod tests {
    use event::log::path::parse_target_path;
    use event::{Bucket, LogRecord, tags};
    use value::value;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn record() {
        let cases = [
            // name, config, logs, want
            (
                "sample_counter",
                MetricConfig {
                    name: "sample_counter".to_string(),
                    field: parse_target_path("foo").unwrap(),
                    tags: Default::default(),
                    typ: MetricType::Counter {
                        increment_by_value: false,
                    },
                },
                vec![value!({"foo": "bar"})],
                vec![
                    // name, tags, value
                    ("sample_counter", tags!(), MetricValue::Sum(2.0)),
                ],
            ),
            (
                "sample_counter_with_increase_by_value",
                MetricConfig {
                    name: "test".into(),
                    field: parse_target_path("foo").unwrap(),
                    tags: Default::default(),
                    typ: MetricType::Counter {
                        increment_by_value: true,
                    },
                },
                vec![
                    // This fields can't be extract, it should be ignored
                    value!({"foo": "bar"}),
                    value!({"foo": "1.2"}),
                    value!({"foo": 2i64}),
                    value!({"foo": 3u64}),
                    value!({"foo": 4.3}),
                ],
                vec![("test", tags!(), MetricValue::Sum(10.5))],
            ),
            (
                "sample_counter_with_tags_and_complex_field",
                MetricConfig {
                    name: "test".to_string(),
                    field: parse_target_path("foo.bar").unwrap(),
                    tags: {
                        let mut map = BTreeMap::new();

                        map.insert("tag1".into(), parse_target_path("tag1").unwrap());
                        map.insert("tag2".into(), parse_target_path("tags.k1").unwrap());
                        map.insert("tag3".into(), parse_target_path("tags.k2").unwrap());

                        map
                    },
                    typ: MetricType::Counter {
                        increment_by_value: false,
                    },
                },
                vec![
                    value!({}),
                    value!({
                        "tag1": "tv1",
                        "tags": {
                            "k1": "v1",
                            "k2": "v2",
                        },
                        "foo": {
                            "bar": 1.23
                        }
                    }),
                ],
                vec![(
                    "test",
                    tags!(
                        "tag1" => "tv1",
                        "tag2" => "v1",
                        "tag3" => "v2"
                    ),
                    MetricValue::Sum(1.0),
                )],
            ),
            (
                "gauge",
                MetricConfig {
                    name: "test".into(),
                    field: parse_target_path("foo").unwrap(),
                    tags: Default::default(),
                    typ: MetricType::Gauge,
                },
                vec![value!({"foo": "1"}), value!({"foo": 2.1})],
                vec![("test", tags!(), MetricValue::Gauge(2.1))],
            ),
            (
                "histogram",
                MetricConfig {
                    name: "test".to_string(),
                    field: parse_target_path("foo").unwrap(),
                    tags: Default::default(),
                    typ: MetricType::Histogram {
                        buckets: default_buckets(),
                    },
                },
                vec![value!({"foo": 0.0005}), value!({"foo": "5"})],
                vec![(
                    "test",
                    tags!(),
                    MetricValue::Histogram {
                        count: 2,
                        sum: 5.0005,
                        buckets: vec![
                            Bucket {
                                count: 1,
                                upper: 0.005,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.01,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.025,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.05,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.1,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.25,
                            },
                            Bucket {
                                count: 1,
                                upper: 0.5,
                            },
                            Bucket {
                                count: 1,
                                upper: 1.0,
                            },
                            Bucket {
                                count: 1,
                                upper: 2.5,
                            },
                            Bucket {
                                count: 2,
                                upper: 5.0,
                            },
                            Bucket {
                                count: 2,
                                upper: 10.0,
                            },
                        ],
                    },
                )],
            ),
        ];

        for (test, config, logs, wants) in cases {
            let mut agg = Metricalize::new(default_interval(), vec![config]);

            let logs = logs.into_iter().map(LogRecord::from).collect::<Vec<_>>();
            agg.record(logs.into());

            let mut output = vec![];
            agg.flush_into(&mut output);

            assert_eq!(
                output.len(),
                wants.len(),
                "metrics count not match, case: {test}"
            );
            #[allow(unused_variables)] // want_value did used
            for (got, (want_name, want_tags, want_value)) in output.iter().zip(wants) {
                assert_eq!(got.name(), want_name, "case: {test}");
                assert_eq!(got.tags(), &want_tags, "case: {test}");
                assert!(matches!(&got.value, want_value), "case: {test}");
            }
        }
    }

    #[test]
    fn test_build_series_and_value() {
        let config = MetricConfig {
            name: "name".to_string(),
            field: parse_target_path("value").unwrap(),
            tags: Default::default(),
            typ: MetricType::Counter {
                increment_by_value: false,
            },
        };

        let (series, value) = config
            .build_series_and_value(&LogRecord::from(value!({ "value" : "a"})))
            .unwrap();

        assert_eq!(series.name, "name");
        assert_eq!(value, 1.0);

        let config = MetricConfig {
            name: "name".to_string(),
            field: parse_target_path("a.b").unwrap(),
            tags: Default::default(),
            typ: MetricType::Counter {
                increment_by_value: false,
            },
        };

        let (series, value) = config
            .build_series_and_value(&LogRecord::from(value!({ "a": { "b": 1 } })))
            .unwrap();
        assert_eq!(series.name, "name");
        assert_eq!(value, 1.0);
    }
}
