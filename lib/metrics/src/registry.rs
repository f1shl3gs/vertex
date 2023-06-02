use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;

use crate::attributes::{assert_legal_key, Attributes};
use crate::metric::{Metric, MetricObserver, Observation};
use crate::Counter;
use crate::Gauge;
use crate::Histogram;

static GLOBAL_REGISTRY: OnceLock<Registry> = OnceLock::new();

#[derive(Clone, Default)]
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

pub fn global_registry() -> Registry {
    GLOBAL_REGISTRY.get_or_init(Registry::new).clone()
}

pub fn register_counter(name: &'static str, description: &'static str) -> Metric<Counter> {
    GLOBAL_REGISTRY
        .get_or_init(Registry::new)
        .register_counter(name, description)
}

pub fn register_gauge(name: &'static str, description: &'static str) -> Metric<Gauge> {
    GLOBAL_REGISTRY
        .get_or_init(Registry::new)
        .register_gauge(name, description)
}

pub fn register_histogram(
    name: &'static str,
    description: &'static str,
    buckets: impl Iterator<Item = f64>,
) -> Metric<Histogram> {
    GLOBAL_REGISTRY
        .get_or_init(Registry::new)
        .register_histogram(name, description, buckets)
}

// RwLock is not used, case most time we don't read SUB_REGISTRIES
static SUB_REGISTRIES: OnceLock<Mutex<BTreeMap<&'static str, Registry>>> = OnceLock::new();

#[derive(Default)]
pub struct SubRegistry {
    key: &'static str,
    registry: Registry,
}

impl Drop for SubRegistry {
    fn drop(&mut self) {
        SUB_REGISTRIES
            .get()
            .expect("SUB_REGISTRY should be init already")
            .lock()
            .remove(self.key);
    }
}

impl SubRegistry {
    pub fn key(&self) -> &'static str {
        self.key
    }

    pub fn register_counter(
        &self,
        name: &'static str,
        description: &'static str,
    ) -> Metric<Counter> {
        self.registry.register_counter(name, description)
    }

    pub fn register_gauge(&self, name: &'static str, description: &'static str) -> Metric<Gauge> {
        self.registry.register_gauge(name, description)
    }

    pub fn register_histogram(
        &self,
        name: &'static str,
        description: &'static str,
        buckets: impl Iterator<Item = f64>,
    ) -> Metric<Histogram> {
        self.registry.register_histogram(name, description, buckets)
    }
}

pub fn sub_registry(key: impl Into<&'static str>) -> Arc<SubRegistry> {
    let key = key.into();

    let registry = SUB_REGISTRIES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .entry(key)
        .or_insert_with(Registry::default)
        .clone();

    Arc::new(SubRegistry { key, registry })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attributes::Attributes;
    use crate::histogram::exponential_buckets;
    use crate::metric::Observation;

    #[test]
    fn register_multiple_times() {
        let reg = Registry::new();

        let cs = reg.register_counter("name", "desc");
        let c1 = cs.recorder(&[("foo", "bar")]);
        assert_eq!(c1.fetch(), 0);
        c1.inc(1);
        assert_eq!(c1.fetch(), 1);

        let cs = reg.register_counter("name", "desc");
        let c2 = cs.recorder(&[("foo", "bar")]);
        assert_eq!(c2.fetch(), 1);
        c2.inc(1);
        assert_eq!(c1.fetch(), 2);
    }

    fn attrs_to_string(attrs: &Attributes) -> String {
        if attrs.is_empty() {
            return String::new();
        }

        let s = attrs.iter().fold("".to_string(), |acc, (k, v)| {
            if acc.is_empty() {
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

        #[allow(clippy::print_stdout)]
        impl Reporter for StdoutReporter {
            fn start_metric(&mut self, name: &'static str, description: &'static str) {
                println!("# {}", name);
                println!("# {}", description);
                self.reporting = Some((name, description))
            }

            fn report(&mut self, attrs: &Attributes, observation: Observation) {
                let (name, _description) = self.reporting.unwrap();

                match observation {
                    Observation::Counter(v) => {
                        println!("{}{} {}", name, attrs_to_string(attrs), v)
                    }
                    Observation::Gauge(v) => {
                        println!("{}{} {}", name, attrs_to_string(attrs), v)
                    }
                    Observation::Histogram(h) => {
                        let mut count = 0;
                        h.buckets.iter().for_each(|b| {
                            count += b.count;

                            let mut sa = attrs.clone();
                            let le = if b.le == f64::MAX {
                                "+Inf".to_string()
                            } else {
                                b.le.to_string()
                            };

                            sa.insert("le", le);
                            println!("{}{} {}", name, attrs_to_string(&sa), count)
                        });

                        println!("{}_sum{} {}", name, attrs_to_string(attrs), h.sum);
                        println!("{}_count{} {}", name, attrs_to_string(attrs), count);
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

    #[test]
    fn test_global() {
        let reg = global_registry();
        let cs = reg.register_counter("c", "d");
        let c1 = cs.recorder(&[]);
        assert_eq!(c1.fetch(), 0);
        c1.inc(1);
        assert_eq!(c1.fetch(), 1);
    }

    #[test]
    fn test_sub_registry() {
        let reg = sub_registry("foo");

        assert_eq!(reg.key(), "foo");

        let cs = reg.register_counter("counter", "counter desc");
        let c = cs.recorder(&[]);
        assert_eq!(c.fetch(), 0);
        c.inc(1);
        assert_eq!(c.fetch(), 1);

        let gs = reg.register_gauge("gauge", "gauge desc");
        let g = gs.recorder(&[]);
        assert_eq!(g.fetch(), 0.0);
        g.inc(1);
        assert_eq!(g.fetch(), 1.0);

        let hs = reg.register_histogram(
            "histogram",
            "histogram desc",
            exponential_buckets(1.0, 2.0, 10),
        );
        let h = hs.recorder(&[]);
        let ho = h.get();
        assert_eq!(ho.sum, 0.0);
        h.record(2.0);
        let ho = h.get();
        assert_eq!(ho.sum, 2.0);
    }
}
