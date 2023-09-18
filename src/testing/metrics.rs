use event::{Bucket, Metric};
use metrics::{global_registry, Attributes, Observation};
use std::collections::BTreeMap;

pub fn capture_metrics() -> Vec<Metric> {
    let registry = global_registry();
    let mut reporter = Reporter::default();

    registry.report(&mut reporter);

    reporter.metrics
}

#[derive(Default)]
struct Reporter {
    inflight: Option<(&'static str, &'static str)>,
    metrics: Vec<Metric>,
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
            Observation::Counter(c) => Metric::sum_with_tags(name, description, c, tags),
            Observation::Gauge(g) => Metric::gauge_with_tags(name, description, g, tags),
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
