use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::pin::Pin;
use std::time::Duration;

use async_stream::stream;
use async_trait::async_trait;
use event::{Bucket, Event, EventMetadata, Metric, MetricSeries, MetricValue, Value};
use framework::config::{
    default_interval, deserialize_duration, serialize_duration, DataType, GenerateConfig, Output,
    TransformConfig, TransformContext, TransformDescription,
};
use framework::{TaskTransform, Transform};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

const fn default_increase_by_value() -> bool {
    false
}

fn default_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ]
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CounterConfig {
    name: String,
    field: String,
    #[serde(default)]
    tags: BTreeMap<String, String>,
    #[serde(default = "default_increase_by_value")]
    increment_by_value: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GaugeConfig {
    name: String,
    field: String,
    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct HistogramConfig {
    name: String,
    field: String,
    #[serde(default)]
    tags: BTreeMap<String, String>,
    #[serde(default = "default_buckets")]
    buckets: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum MetricConfig {
    Counter(CounterConfig),
    Gauge(GaugeConfig),
    Histogram(HistogramConfig),
}

impl MetricConfig {
    fn build_series_and_value(
        &self,
        fields: &BTreeMap<String, Value>,
    ) -> Option<(MetricSeries, f64)> {
        let (name, tags, field, parse_value) = match self {
            MetricConfig::Counter(config) => (
                &config.name,
                &config.tags,
                &config.field,
                config.increment_by_value,
            ),
            MetricConfig::Histogram(config) => (&config.name, &config.tags, &config.field, true),
            MetricConfig::Gauge(config) => (&config.name, &config.tags, &config.field, true),
        };

        let value = match event::log::get::get(fields, field)? {
            Value::Int64(i) => *i as f64,
            Value::Uint64(u) => *u as f64,
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

        let mut t = BTreeMap::new();
        for (k, v) in tags {
            let value = match event::log::get::get(fields, v) {
                Some(value) => value.to_string_lossy(),
                None => String::new(),
            };

            t.insert(k.to_string(), value);
        }

        Some((
            MetricSeries {
                name: name.to_string(),
                tags: t,
            },
            value,
        ))
    }

    fn new_metric_value(&self, value: f64) -> MetricValue {
        match self {
            MetricConfig::Counter(_) => MetricValue::Sum(value),
            MetricConfig::Gauge(_) => MetricValue::Gauge(value),
            MetricConfig::Histogram(config) => MetricValue::Histogram {
                count: 1,
                sum: value,
                buckets: config
                    .buckets
                    .iter()
                    .map(|upper| Bucket {
                        upper: *upper,
                        count: if value <= *upper { 1 } else { 0 },
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AggregateConfig {
    #[serde(
        default = "default_interval",
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
    metrics: Vec<MetricConfig>,
}

impl GenerateConfig for AggregateConfig {
    fn generate_config() -> String {
        r#"
# The interval between flushes.
#
# interval: 15s

# A table of key/value pairs representing the keys to be added to the event.
#
metrics:
# Metric type
#
# Availabel values
# counter:     Counter
# gauge:       Gauge
# histogram:   Histogram
#
- type: counter
  # Metric name, it's highly recomment to see
  # https://prometheus.io/docs/practices/naming/
  #
  name: some_error_total

  # Which field to extract values
  #
  # Path is support too, e.g.
  # field: value.i64
  field: value

  # Tags to set, this field is not required,
  # but is is recomment to set some tags to identify your metrics.
  #
  # tags:
  #   foo: bar
  #   hostname: ${ HOSTNAME }
  #   inner: some.inner1.array[0]

  # Controls how to increase the counter.
  # Available for "counter" only.
  #
  # increase_by_value: false

  # Specify histogram buckets.
  # Available for "histogram" only
  # Default: 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
  #
  # buckets:
  # - 0.1
  # - 0.2
"#
        .into()
    }
}

inventory::submit! {
    TransformDescription::new::<AggregateConfig>("aggregate")
}

#[async_trait]
#[typetag::serde(name = "aggregate")]
impl TransformConfig for AggregateConfig {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let agg = Aggregate::new(self.interval, self.metrics.clone());
        Ok(Transform::event_task(agg))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn transform_type(&self) -> &'static str {
        "aggregate"
    }
}

impl TaskTransform for Aggregate {
    fn transform(
        mut self: Box<Self>,
        mut input_rx: Pin<Box<dyn Stream<Item = Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        let mut ticker = tokio::time::interval(self.interval);

        Box::pin(stream! {
            let mut output = Vec::new();
            let mut done = false;

            while !done {
                tokio::select! {
                    _ = ticker.tick() => {
                        self.flush_into(&mut output);
                    },

                    maybe_event = input_rx.next() => {
                        match maybe_event {
                            Some(event) => self.record(event),
                            None => {
                                self.flush_into(&mut output);
                                done = true;
                            }
                        }
                    }
                };

                for event in output.drain(..) {
                    yield event;
                }
            }
        })
    }
}

type MetricEntry = (MetricValue, EventMetadata);

struct Aggregate {
    interval: Duration,
    configs: Vec<MetricConfig>,
    states: HashMap<MetricSeries, MetricEntry>,
}

impl Aggregate {
    fn new(interval: Duration, configs: Vec<MetricConfig>) -> Self {
        Self {
            interval,
            configs,
            states: Default::default(),
        }
    }

    fn record(&mut self, event: Event) {
        let (_, fields, metadata) = event.into_log().into_parts();

        for config in &self.configs {
            match config.build_series_and_value(&fields) {
                Some((series, value)) => {
                    match self.states.entry(series) {
                        Entry::Occupied(mut entry) => {
                            let existing = entry.get_mut();

                            // In order to update the new and old kind must match
                            match (&existing.0, config) {
                                (MetricValue::Sum(_), MetricConfig::Counter(_))
                                | (MetricValue::Gauge(_), MetricConfig::Gauge(_))
                                | (MetricValue::Histogram { .. }, MetricConfig::Histogram(_)) => {
                                    existing.0.merge(value);
                                    existing.1.merge(metadata.clone());
                                }
                                _ => {
                                    *existing = (config.new_metric_value(value), metadata.clone());
                                    counter!("aggregate_failed_total", 1);
                                }
                            }
                        }

                        Entry::Vacant(entry) => {
                            entry.insert((config.new_metric_value(value), metadata.clone()));
                        }
                    }
                }
                None => counter!("aggregate_failed_total", 1),
            }
        }
    }

    fn flush_into(&mut self, output: &mut Vec<Event>) {
        for (series, entry) in self.states.drain() {
            let metric =
                Metric::new_with_metadata(series.name, series.tags, entry.0, None, entry.1);

            output.push(metric.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::{fields, tags, Bucket};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<AggregateConfig>()
    }

    #[test]
    fn record() {
        let cases = [
            // name, config, logs, want
            (
                "sample_counter",
                MetricConfig::Counter(CounterConfig {
                    name: "sample_counter".to_string(),
                    field: "foo".to_string(),
                    tags: Default::default(),
                    increment_by_value: false,
                }),
                vec![fields!("foo" => "bar")],
                vec![
                    // name, tags, value
                    ("sample_counter", tags!(), MetricValue::Sum(2.0)),
                ],
            ),
            (
                "sample_counter_with_increase_by_value",
                MetricConfig::Counter(CounterConfig {
                    name: "test".into(),
                    field: "foo".into(),
                    tags: Default::default(),
                    increment_by_value: true,
                }),
                vec![
                    // This fields can't be extract, it should be ignored
                    fields!("foo" => "bar"),
                    fields!("foo" => "1.2"),
                    fields!("foo" => 2i64),
                    fields!("foo" => 3u64),
                    fields!("foo" => 4.3),
                ],
                vec![("test", tags!(), MetricValue::Sum(10.5))],
            ),
            (
                "sample_counter_with_tags_and_complex_field",
                MetricConfig::Counter(CounterConfig {
                    name: "test".to_string(),
                    field: "foo.bar".to_string(),
                    tags: tags!(
                        "tag1" => "tag1",
                        "tag2" => "tags.k1",
                        "tag3" => "tags.k2"
                    ),
                    increment_by_value: false,
                }),
                vec![
                    fields!(),
                    fields!(
                        "tag1" => "tv1",
                        "tags" => fields!(
                            "k1" => "v1",
                            "k2" => "v2",
                        ),
                        "foo" => fields!(
                            "bar" => 1.23
                        )
                    ),
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
                MetricConfig::Gauge(GaugeConfig {
                    name: "test".into(),
                    field: "foo".to_string(),
                    tags: Default::default(),
                }),
                vec![fields!("foo" => "1"), fields!("foo" => 2.1)],
                vec![("test", tags!(), MetricValue::Gauge(2.1))],
            ),
            (
                "histogram",
                MetricConfig::Histogram(HistogramConfig {
                    name: "test".to_string(),
                    field: "foo".to_string(),
                    tags: Default::default(),
                    buckets: default_buckets(),
                }),
                vec![fields!("foo" => 0.0005), fields!("foo" => "5")],
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
            let mut agg = Aggregate::new(default_interval(), vec![config]);

            for log in logs {
                agg.record(Event::from(log));
            }

            let mut output = vec![];
            agg.flush_into(&mut output);

            assert_eq!(output.len(), wants.len(), "case: {}", test);
            for (got, (want_name, want_tags, want_value)) in output.iter().zip(wants) {
                let got = got.as_metric();
                assert_eq!(got.name(), want_name, "case: {}", test);
                assert_eq!(got.tags(), &want_tags, "case: {}", test);
                assert!(matches!(&got.value, want_value), "case: {}", test);
            }
        }
    }
}
