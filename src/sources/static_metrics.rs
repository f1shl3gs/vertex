use std::borrow::Cow;
use std::time::Duration;

use chrono::Utc;
use configurable::{Configurable, configurable_component};
use event::tags::Tags;
use event::{Metric, MetricValue};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use serde::{Deserialize, Serialize};

/// Configuration for each static metric
#[derive(Clone, Debug, Deserialize, Serialize, Configurable)]
struct StaticMetricConfig {
    /// Name of the static metric
    name: String,

    /// Description of this static metric
    #[serde(default)]
    description: Option<String>,

    /// Key-value pairs representing tags and their values to add to the metric.
    #[serde(skip_serializing_if = "Tags::is_empty")]
    tags: Tags,

    /// "Observed" value of the static metric
    value: MetricValue,
}

/// Produce static metrics defined in configuration.
#[configurable_component(source, name = "static_metrics")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The interval between metric emitting
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Metrics to produce
    #[serde(default)]
    metrics: Vec<StaticMetricConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "static_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            self.interval,
            self.metrics.clone(),
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    interval: Duration,
    metric_configs: Vec<StaticMetricConfig>,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            // The first tick completes immediately
            _ = ticker.tick() => {},
            _ = &mut shutdown => {
                break
            }
        }

        // generate metrics
        let ts = Utc::now();
        let metrics = metric_configs
            .iter()
            .map(|conf| {
                Metric::new(
                    conf.name.clone(),
                    conf.description.clone().map(Cow::Owned),
                    conf.tags.clone(),
                    ts,
                    conf.value.clone(),
                )
            })
            .collect::<Vec<_>>();

        if let Err(err) = output.send(metrics).await {
            warn!(message = "failed to send metrics", ?err);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::components::{SOURCE_TAGS, run_and_assert_source_compliance};
    use event::{Bucket, Quantile, tags};

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    async fn events_from_config(config: Config) -> Vec<Metric> {
        run_and_assert_source_compliance(config, Duration::from_millis(100), &SOURCE_TAGS)
            .await
            .into_iter()
            .flat_map(|events| events.into_metrics())
            .flatten()
            .collect()
    }

    #[tokio::test]
    async fn default_empty() {
        let events = events_from_config(Config {
            interval: default_interval(),
            metrics: vec![],
        })
        .await;

        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn multiple_metrics_with_tags() {
        let events = events_from_config(Config {
            interval: default_interval(),
            metrics: vec![
                StaticMetricConfig {
                    name: "gauge".to_string(),
                    description: None,
                    tags: tags!("foo" => "bar"),
                    value: MetricValue::Gauge(1.1),
                },
                StaticMetricConfig {
                    name: "sum".to_string(),
                    description: Some("sum description".to_string()),
                    tags: tags!("foo" => "bar"),
                    value: MetricValue::Sum(2.2),
                },
                StaticMetricConfig {
                    name: "histogram".to_string(),
                    description: Some("histogram".to_string()),
                    tags: tags!("bar" => "foo"),
                    value: MetricValue::Histogram {
                        count: 2,
                        sum: 2.2,
                        buckets: vec![
                            Bucket {
                                upper: 1.0,
                                count: 1,
                            },
                            Bucket {
                                upper: 2.0,
                                count: 1,
                            },
                        ],
                    },
                },
                StaticMetricConfig {
                    name: "summary".to_string(),
                    description: Some("summary".to_string()),
                    tags: tags!("bar" => "foo"),
                    value: MetricValue::Summary {
                        count: 3,
                        sum: 3.3,
                        quantiles: vec![
                            Quantile {
                                quantile: 1.1,
                                value: 1.1,
                            },
                            Quantile {
                                quantile: 2.2,
                                value: 2.2,
                            },
                            Quantile {
                                quantile: 3.3,
                                value: 3.3,
                            },
                        ],
                    },
                },
            ],
        })
        .await;

        let want = [
            (
                "gauge",
                None,
                tags!("foo" => "bar"),
                MetricValue::Gauge(1.1),
            ),
            (
                "sum",
                Some("sum description".into()),
                tags!("foo" => "bar"),
                MetricValue::Sum(2.2),
            ),
            (
                "histogram",
                Some("histogram".into()),
                tags!("bar" => "foo"),
                MetricValue::Histogram {
                    count: 2,
                    sum: 2.2,
                    buckets: vec![
                        Bucket {
                            upper: 1.0,
                            count: 1,
                        },
                        Bucket {
                            upper: 2.0,
                            count: 1,
                        },
                    ],
                },
            ),
            (
                "summary",
                Some("summary".into()),
                tags!("bar" => "foo"),
                MetricValue::Summary {
                    count: 3,
                    sum: 3.3,
                    quantiles: vec![
                        Quantile {
                            quantile: 1.1,
                            value: 1.1,
                        },
                        Quantile {
                            quantile: 2.2,
                            value: 2.2,
                        },
                        Quantile {
                            quantile: 3.3,
                            value: 3.3,
                        },
                    ],
                },
            ),
        ];

        assert_eq!(events.len(), want.len());
        for (got, (name, description, tags, value)) in events.into_iter().zip(want) {
            assert_eq!(got.name(), name);
            assert_eq!(got.description, description);
            assert_eq!(got.tags(), &tags);
            assert_eq!(got.value(), &value);
        }
    }
}
