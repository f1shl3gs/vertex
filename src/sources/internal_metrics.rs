use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Utc;
use configurable::configurable_component;
use event::Bucket;
use framework::config::Output;
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{default_interval, DataType, SourceConfig, SourceContext},
    Source,
};
use metrics::{Attributes, Observation};

/// Exposes Vertex's own internal metrics, allowing you to collect,
/// process, and route Vertex's internal metrics just like other metrics.
#[configurable_component(source, name = "internal_metrics")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Duration between reports
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "internal_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(self.interval, cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

async fn run(
    interval: Duration,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        // Report metrics as soon as possible
        let mut reporter = Reporter::default();
        let reg = metrics::global_registry();
        reg.report(&mut reporter);

        // set timestamp
        let timestamp = Utc::now();
        reporter
            .metrics
            .iter_mut()
            .for_each(|metric| metric.timestamp = Some(timestamp));

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

    fn report(&mut self, attrs: &Attributes, observation: Observation) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
