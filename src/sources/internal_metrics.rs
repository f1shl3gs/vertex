use std::collections::BTreeMap;
use std::time::Duration;

use event::Bucket;
use framework::config::{default_interval, GenerateConfig, Output, SourceDescription};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{deserialize_duration, serialize_duration, DataType, SourceConfig, SourceContext},
    Source,
};
use futures::StreamExt;
use metrics::{Attributes, Observation};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InternalMetricsConfig {
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
}

impl GenerateConfig for InternalMetricsConfig {
    fn generate_config() -> String {
        r#"
# The interval between scrapes.
#
# Default: 15s
# interval: 15s
"#
        .into()
    }
}

inventory::submit! {
    SourceDescription::new::<InternalMetricsConfig>("internal_metrics")
}

#[async_trait::async_trait]
#[typetag::serde(name = "internal_metrics")]
impl SourceConfig for InternalMetricsConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(self.interval, cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "internal_metrics"
    }
}

async fn run(interval: Duration, shutdown: ShutdownSignal, mut output: Pipeline) -> Result<(), ()> {
    let interval = tokio::time::interval(interval);
    let mut ticker = IntervalStream::new(interval).take_until(shutdown);

    while ticker.next().await.is_some() {
        let mut reporter = Reporter::default();
        let reg = metrics::global_registry();
        reg.report(&mut reporter);

        if let Err(err) = output.send(reporter.metrics).await {
            error!(
                message = "Error sending internal metrics",
                %err
            );

            return Err(());
        }
    }

    Ok(())
}

#[derive(Default)]
struct Reporter {
    inflight: Option<(&'static str, &'static str)>,
    metrics: Vec<event::Metric>,
}

impl metrics::Reporter for Reporter {
    fn start_metric(&mut self, name: &'static str, description: &'static str) {
        self.inflight = Some((name, description));
    }

    fn report(&mut self, attrs: &Attributes, observation: metrics::Observation) {
        let (name, description) = self
            .inflight
            .expect("name and description should be set already");

        let tags = attrs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>();

        let metric = match observation {
            Observation::Counter(c) => event::Metric::sum_with_tags(name, description, c, tags),
            Observation::Gauge(g) => event::Metric::gauge_with_tags(name, description, g, tags),
            Observation::Histogram(h) => {
                let mut count = 0;
                let buckets = h
                    .buckets
                    .iter()
                    .map(|mb| {
                        count += mb.count;

                        Bucket {
                            upper: mb.le,
                            count,
                        }
                    })
                    .collect::<Vec<_>>();

                event::Metric::histogram_with_tags(name, description, tags, count, h.sum, buckets)
            }
        };

        self.metrics.push(metric)
    }

    fn finish_metric(&mut self) {
        self.inflight = None;
    }
}
