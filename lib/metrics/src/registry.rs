use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::attributes::{assert_legal_key, Attributes};
use crate::counter::Counter;
use crate::gauge::Gauge;
use crate::histogram::Histogram;
use crate::metric::{Metric, MetricObserver, Observation};

pub struct Registry {
    counters: Arc<Mutex<BTreeMap<&'static str, Metric<Counter>>>>,
    gauges: Arc<Mutex<BTreeMap<&'static str, Metric<Gauge>>>>,
    histograms: Arc<Mutex<BTreeMap<&'static str, Metric<Histogram>>>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry {
            counters: Arc::new(Default::default()),
            gauges: Arc::new(Default::default()),
            histograms: Arc::new(Default::default()),
        }
    }

    pub fn register_counter(
        &self,
        name: &'static str,
        description: &'static str,
    ) -> Metric<Counter> {
        assert_legal_key(name);

        self.counters
            .lock()
            .entry(name)
            .or_insert_with(|| Metric {
                name,
                description,
                shard: Arc::new(Mutex::new(BTreeMap::new())),

                // dummy
                options: (),
            })
            .clone()
    }

    pub fn register_gauge(&self, name: &'static str, description: &'static str) -> Metric<Gauge> {
        assert_legal_key(name);

        self.gauges
            .lock()
            .entry(name)
            .or_insert_with(|| Metric {
                name,
                description,
                shard: Arc::new(Mutex::new(BTreeMap::new())),
                options: (),
            })
            .clone()
    }

    pub fn register_histogram(
        &self,
        name: &'static str,
        description: &'static str,
        buckets: impl Iterator<Item = f64>,
    ) -> Metric<Histogram> {
        assert_legal_key(name);

        let options = buckets.collect::<Vec<f64>>();

        self.histograms
            .lock()
            .entry(name)
            .or_insert_with(|| Metric {
                name,
                description,
                shard: Arc::new(Mutex::new(BTreeMap::new())),
                options,
            })
            .clone()
    }

    pub fn report(&self, reporter: &mut impl Reporter) {
        self.report_generic(reporter, Arc::clone(&self.counters));
        self.report_generic(reporter, Arc::clone(&self.gauges));
        self.report_generic(reporter, Arc::clone(&self.histograms));
    }

    fn report_generic<M: MetricObserver>(
        &self,
        reporter: &mut impl Reporter,
        metrics: Arc<Mutex<BTreeMap<&'static str, Metric<M>>>>,
    ) {
        metrics.lock().iter().for_each(|(_, set)| {
            reporter.start_metric(set.name, set.description);
            set.shard
                .lock()
                .iter()
                .for_each(|(attrs, metric)| reporter.report(attrs, metric.observe()));
            reporter.finish_metric()
        })
    }
}

pub trait Reporter {
    fn start_metric(&mut self, name: &'static str, description: &'static str);

    fn report(&mut self, attrs: &Attributes, observation: Observation);

    /// Finish recording a given metric
    fn finish_metric(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::histogram::exponential_buckets;

    #[test]
    fn register_counter() {
        let reg = Registry::new();

        let cs = reg.register_counter("name", "desc");
        let c1 = cs.recorder(&[("foo", "bar")]);
        assert_eq!(c1.fetch(), 0);
        c1.inc(1);
        assert_eq!(c1.fetch(), 1);
    }

    fn attrs_to_string(attrs: &Attributes) -> String {
        if attrs.len() == 0 {
            return String::new();
        }

        let s = attrs.iter().fold("".to_string(), |acc, (k, v)| {
            if acc.len() == 0 {
                format!("{}=\"{}\"", k, v)
            } else {
                format!("{},{}=\"{}\"", acc, k, v)
            }
        });

        format!("{{{}}}", s)
    }

    #[test]
    fn reporter() {
        struct StdoutReporter {
            reporting: Option<(&'static str, &'static str)>,
        }

        impl Reporter for StdoutReporter {
            fn start_metric(&mut self, name: &'static str, description: &'static str) {
                println!("# {}", name);
                println!("# {}", description);
                self.reporting = Some((name, description))
            }

            fn report(&mut self, attrs: &Attributes, observation: Observation) {
                let (name, _description) = self.reporting.unwrap();

                match observation {
                    Observation::Counter(v) | Observation::Gauge(v) => {
                        println!("{} {} {}", name, attrs_to_string(attrs), v)
                    }
                    Observation::Histogram(h) => {
                        h.buckets.iter().for_each(|b| {
                            let mut sa = attrs.clone();
                            let le = if b.le == f64::MAX {
                                "+inf".to_string()
                            } else {
                                b.le.to_string()
                            };

                            sa.insert("le", le);
                            println!("{} {} {}", name, attrs_to_string(&sa), b.count)
                        });

                        println!("{}_sum {} {}", name, attrs_to_string(attrs), h.sum);
                        println!("{}_total {} {}", name, attrs_to_string(attrs), h.count);
                    }
                };
            }

            fn finish_metric(&mut self) {
                self.reporting = None
            }
        }

        let reg = Registry::new();

        let gs = reg.register_gauge("gauge", "gauge desc");
        let g = gs.recorder(&[]);
        g.inc(1);
        let g = gs.recorder(&[("key", "value")]);
        g.inc(2);

        let cs = reg.register_counter("counter", "counter desc");
        let c = cs.recorder(&[]);
        c.inc(2);
        let c = cs.recorder(&[("foo", "bar")]);
        c.inc(2);

        let hs = reg.register_histogram(
            "histogram",
            "histogram description",
            exponential_buckets(1.0, 2.0, 10),
        );
        let h = hs.recorder(&[]);
        h.record(12.0);
        h.record(3.0);
        let h = hs.recorder(&[("key", "value")]);
        h.record(12.0);
        h.record(4.0);

        let mut stdout_reporter = StdoutReporter { reporting: None };

        reg.report(&mut stdout_reporter)
    }
}
